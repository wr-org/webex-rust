//! WebSocket event stream handling for real-time Webex events.

use crate::error::Error;
use crate::types::{Authorization, Event};
use futures_util::{SinkExt, StreamExt};
use log::{debug, trace, warn};
use std::time::Duration;
use tokio_tungstenite::tungstenite::{Error as TErr, Message as TMessage};

const AUTH_TIMEOUT: Duration = Duration::from_secs(20);

/// WebSocket stream type.
pub type WStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// Webex event stream for receiving real-time events via WebSocket.
pub struct WebexEventStream {
    pub(crate) ws_stream: WStream,
    pub(crate) timeout: Duration,
    /// Signifies if `WebStream` is Open
    pub is_open: bool,
}

impl WebexEventStream {
    /// Creates a new `WebexEventStream` from a WebSocket stream.
    pub(crate) const fn new(ws_stream: WStream, timeout: Duration) -> Self {
        Self {
            ws_stream,
            timeout,
            is_open: true,
        }
    }

    /// Get the next event from an event stream.
    ///
    /// Returns an event or an error.
    ///
    /// # Errors
    /// Malformed events return an error without closing the stream. When
    /// [`is_open`](Self::is_open) becomes false, obtain a new stream using
    /// [`Webex::event_stream`](super::Webex::event_stream). Further calls on a
    /// closed stream return [`Error::Closed`].
    pub async fn next(&mut self) -> Result<Event, Error> {
        if !self.is_open {
            return Err(Error::Closed("WebSocket is closed".to_string()));
        }
        loop {
            let next = self.ws_stream.next();

            match tokio::time::timeout(self.timeout, next).await {
                // Timed out
                Err(_) => {
                    // This does not seem to be recoverable, or at least there are conditions under
                    // which it does not recover. Indicate that the connection is closed and a new
                    // one will have to be opened.
                    self.is_open = false;
                    return Err(format!("no activity for at least {:?}", self.timeout).into());
                }
                // Didn't time out
                Ok(next_result) => match next_result {
                    None => {
                        self.is_open = false;
                        return Err(Error::Closed("WebSocket stream ended".to_string()));
                    }
                    Some(msg) => match msg {
                        Ok(msg) => {
                            if let Some(h_msg) = self.handle_message(msg)? {
                                return Ok(h_msg);
                            }
                            // `None` messages still reset the timeout (e.g. Ping to keep alive)
                        }
                        Err(e @ (TErr::Protocol(_) | TErr::Io(_))) => {
                            // Protocol error probably requires a connection reset
                            // IO error is (apart from WouldBlock) generally an error with the
                            // underlying connection and also fatal
                            self.is_open = false;
                            return Err(e.to_string().into());
                        }
                        Err(e) => {
                            self.is_open = false;
                            return Err(Error::Tungstenite(
                                Box::new(e),
                                "Error getting next_result".into(),
                            ));
                        }
                    },
                },
            }
        }
    }

    fn handle_message(&mut self, msg: TMessage) -> Result<Option<Event>, Error> {
        match msg {
            TMessage::Binary(bytes) => {
                let json = std::str::from_utf8(&bytes)?;
                match serde_json::from_str(json) {
                    Ok(ev) => Ok(Some(ev)),
                    Err(e) => {
                        warn!("Couldn't deserialize: {:?}.  Original JSON:\n{}", e, &json);
                        Err(e.into())
                    }
                }
            }
            TMessage::Text(t) => {
                debug!("text: {t}");
                Ok(None)
            }
            TMessage::Ping(_) => {
                trace!("Ping!");
                Ok(None)
            }
            TMessage::Close(t) => {
                debug!("close: {t:?}");
                self.is_open = false;
                Err(Error::Closed("Web Socket Closed".to_string()))
            }
            TMessage::Pong(_) => {
                debug!("Pong!");
                Ok(None)
            }
            TMessage::Frame(_) => {
                debug!("Frame");
                Ok(None)
            }
        }
    }

    /// Authenticate to the WebSocket stream.
    pub(crate) async fn auth(ws_stream: &mut WStream, token: &str) -> Result<(), Error> {
        Self::auth_with_timeout(ws_stream, token, AUTH_TIMEOUT).await
    }

    /// Bound both sending credentials and waiting for the authentication reply.
    async fn auth_with_timeout(
        ws_stream: &mut WStream,
        token: &str,
        timeout: Duration,
    ) -> Result<(), Error> {
        tokio::time::timeout(timeout, Self::authenticate(ws_stream, token))
            .await
            .map_err(|_| {
                Error::Io(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "WebSocket authentication timed out",
                ))
            })?
    }

    async fn authenticate(ws_stream: &mut WStream, token: &str) -> Result<(), Error> {
        let auth = Authorization::new(token);
        debug!("Authenticating to stream");
        let auth_json = serde_json::to_string(&auth)?;
        match ws_stream.send(TMessage::Text(auth_json.into())).await {
            Ok(()) => {
                // The next thing back should be a pong
                match ws_stream.next().await {
                    Some(msg) => match msg {
                        Ok(msg) => match msg {
                            TMessage::Ping(_) | TMessage::Pong(_) => {
                                debug!("Authentication succeeded");
                                Ok(())
                            }
                            _ => Err(format!("Received {msg:?} in reply to auth message").into()),
                        },
                        Err(e) => Err(format!("Received error from websocket: {e}").into()),
                    },
                    None => Err("Websocket closed".to_string().into()),
                }
            }
            Err(e) => Err(Error::Tungstenite(
                Box::new(e),
                "failed to send authentication".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::{TcpListener, TcpStream};
    use tokio_tungstenite::{accept_async, connect_async, WebSocketStream};

    async fn socket_pair() -> (WStream, WebSocketStream<TcpStream>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}", listener.local_addr().unwrap());
        let connect = connect_async(url);
        let accept = async {
            let (socket, _) = listener.accept().await.unwrap();
            accept_async(socket).await.unwrap()
        };
        let (client, server) = tokio::time::timeout(Duration::from_secs(2), async {
            tokio::join!(connect, accept)
        })
        .await
        .unwrap();
        (client.unwrap().0, server)
    }

    #[tokio::test]
    async fn exhausted_stream_returns_closed_repeatedly() {
        let (mut client, mut server) = socket_pair().await;
        server.close(None).await.unwrap();
        assert!(matches!(client.next().await, Some(Ok(TMessage::Close(_)))));
        client.flush().await.unwrap();
        assert!(matches!(server.next().await, Some(Ok(TMessage::Close(_)))));
        assert!(server.next().await.is_none());
        drop(server);
        assert!(client.next().await.is_none());

        let mut stream = WebexEventStream::new(client, Duration::from_secs(1));
        for _ in 0..3 {
            assert!(matches!(stream.next().await, Err(Error::Closed(_))));
            assert!(!stream.is_open);
        }
    }

    #[tokio::test]
    async fn close_frame_returns_closed_repeatedly() {
        let (client, mut server) = socket_pair().await;
        server.close(None).await.unwrap();
        let mut stream = WebexEventStream::new(client, Duration::from_secs(1));
        for _ in 0..3 {
            assert!(matches!(stream.next().await, Err(Error::Closed(_))));
            assert!(!stream.is_open);
        }
    }

    #[tokio::test]
    async fn idle_timeout_marks_stream_closed() {
        let (client, _server) = socket_pair().await;
        let mut stream = WebexEventStream::new(client, Duration::from_millis(30));
        assert!(stream.next().await.is_err());
        assert!(!stream.is_open);
        assert!(matches!(stream.next().await, Err(Error::Closed(_))));
    }

    #[tokio::test]
    async fn malformed_event_does_not_close_stream() {
        let (client, mut server) = socket_pair().await;
        server
            .send(TMessage::Binary(b"invalid json".to_vec().into()))
            .await
            .unwrap();
        server
            .send(TMessage::Binary(
                serde_json::to_vec(&Event::default()).unwrap().into(),
            ))
            .await
            .unwrap();
        let mut stream = WebexEventStream::new(client, Duration::from_secs(1));
        assert!(matches!(stream.next().await, Err(Error::Json(_))));
        assert!(stream.is_open);
        assert_eq!(stream.next().await.unwrap(), Event::default());
    }

    #[tokio::test]
    async fn authentication_times_out_with_a_silent_peer() {
        let (mut client, mut server) = socket_pair().await;
        let result = WebexEventStream::auth_with_timeout(
            &mut client,
            "test-token",
            Duration::from_millis(30),
        )
        .await;
        assert!(
            matches!(result, Err(Error::Io(ref error)) if error.kind() == std::io::ErrorKind::TimedOut)
        );
        assert!(matches!(server.next().await, Some(Ok(TMessage::Text(_)))));
    }

    #[tokio::test]
    async fn authentication_accepts_ping_and_pong() {
        for response in [TMessage::Ping(vec![].into()), TMessage::Pong(vec![].into())] {
            let (mut client, mut server) = socket_pair().await;
            let authenticate = WebexEventStream::auth(&mut client, "test-token");
            let reply = async {
                let Some(Ok(TMessage::Text(text))) = server.next().await else {
                    panic!("expected an authentication message");
                };
                let auth: serde_json::Value = serde_json::from_str(&text).unwrap();
                assert_eq!(auth["data"]["token"], "Bearer test-token");
                server.send(response).await.unwrap();
            };
            let (result, ()) = tokio::join!(authenticate, reply);
            result.unwrap();
        }
    }

    #[tokio::test]
    async fn authentication_rejects_unexpected_replies() {
        let (mut client, mut server) = socket_pair().await;
        let authenticate = WebexEventStream::auth(&mut client, "test-token");
        let reply = async {
            assert!(matches!(server.next().await, Some(Ok(TMessage::Text(_)))));
            server
                .send(TMessage::Text("unexpected".into()))
                .await
                .unwrap();
        };
        let (result, ()) = tokio::join!(authenticate, reply);
        assert!(result.is_err());
    }
}
