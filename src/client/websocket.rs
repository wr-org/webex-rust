//! WebSocket event stream handling for real-time Webex events.

use crate::error::Error;
use crate::types::{Authorization, Event};
use futures_util::{SinkExt, StreamExt};
use log::{debug, trace, warn};
use std::time::Duration;
use tokio_tungstenite::tungstenite::{Error as TErr, Message as TMessage};

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
    /// Returns an error when the underlying stream has a problem, but will
    /// continue to work on subsequent calls to `next()` - the errors can safely
    /// be ignored.
    pub async fn next(&mut self) -> Result<Event, Error> {
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
                    None => {}
                    Some(msg) => match msg {
                        Ok(msg) => {
                            if let Some(h_msg) = self.handle_message(msg)? {
                                return Ok(h_msg);
                            }
                            // `None` messages still reset the timeout (e.g. Ping to keep alive)
                        }
                        Err(TErr::Protocol(_) | TErr::Io(_)) => {
                            // Protocol error probably requires a connection reset
                            // IO error is (apart from WouldBlock) generally an error with the
                            // underlying connection and also fatal
                            self.is_open = false;
                            return Err(msg.unwrap_err().to_string().into());
                        }
                        Err(e) => {
                            return Err(Error::Tungstenite(
                                Box::new(e),
                                "Error getting next_result".into(),
                            ))
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
