pub mod adaptive_card;
pub mod types;
use futures::{SinkExt, StreamExt};
use hyper::{body::HttpBody, client::HttpConnector, Body, Client, Request};
use hyper_tls::HttpsConnector;
use log::{debug, warn};
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::HashMap, time::Duration};
use tokio::net::TcpStream;
use tokio_tls::TlsStream;
use tokio_tungstenite::stream::Stream;
use tokio_tungstenite::{connect_async, WebSocketStream};
use tungstenite::protocol::Message;
use uuid::Uuid;

/*
 * URLs:
 *
 * https://help.webex.com/en-us/xbcr37/External-Connections-Made-by-the-Serviceability-Connector
 *
 * These apply to the central Webex Teams (Wxt) servers.  WxT also supports enterprise servers;
 * these are not supported.
 */

const REST_HOST_PREFIX: &str = "https://api.ciscospark.com/v1";
const REGISTRATION_HOST_PREFIX: &str = "https://wdm-a.wbx2.com/wdm/api/v1";

pub type WStream = WebSocketStream<Stream<TcpStream, TlsStream<TcpStream>>>;
type WebClient = Client<HttpsConnector<HttpConnector>, Body>;

#[derive(Clone)]
pub struct Webex {
    client: WebClient,
    bearer: String,
    token: String,
    host_prefix: HashMap<String, String>,
}

pub struct WebexEventStream {
    ws_stream: WStream,
    timeout: Duration,
}

impl WebexEventStream {
    /// Get the next event from an event stream
    ///
    /// Returns an event or an error
    pub async fn next(&mut self) -> Result<types::Event, String> {
        loop {
            let next = self.ws_stream.next();
            match tokio::time::timeout(self.timeout, next).await {
                // Timed out
                Err(_) => return Err(format!("no activity for at least {:?}", self.timeout)),
                // Didn't time out
                Ok(next_result) => match next_result {
                    Some(msg) => match msg {
                        Ok(msg) => {
                            match self.handle_message(msg).await {
                                Ok(maybe_msg) => {
                                    if let Some(msg) = maybe_msg {
                                        return Ok(msg);
                                    } else {
                                        // Ignore other messages (but they'll reset the timeout)
                                        continue;
                                    }
                                }
                                Err(e) => return Err(e),
                            };
                        }
                        Err(e) => return Err(e.to_string()),
                    },
                    None => continue,
                },
            }
        }
    }

    async fn handle_message(&self, msg: Message) -> Result<Option<types::Event>, String> {
        match msg {
            Message::Binary(bytes) => match String::from_utf8(bytes) {
                Ok(json) => {
                    let json = json.as_str();
                    let event: Result<types::Event, _> = serde_json::from_str(json);
                    match event {
                        Ok(event) => Ok(Some(event)),
                        Err(e) => {
                            warn!("Couldn't deserialize: {:?}.  Original JSON:\n{}", e, &json);
                            Err(format!("unable to deserialize: {}", e))
                        }
                    }
                }
                Err(e) => Err(format!("UTF-8 decode failed: {}", e)),
            },
            Message::Text(t) => {
                debug!("text: {}", t);
                Ok(None)
            }
            Message::Ping(_) => {
                debug!("Ping!");
                Ok(None)
            }
            Message::Close(t) => {
                debug!("close: {:?}", t);
                Ok(None)
            }
            Message::Pong(_) => {
                debug!("Pong!");
                Ok(None)
            }
        }
    }
}

impl Webex {
    /// Constructs a new Webex Teams context
    pub fn new(token: &str) -> Self {
        let https = HttpsConnector::new();
        let client = Client::builder().build::<_, hyper::Body>(https);

        let mut webex = Webex {
            client,
            token: token.to_string(),
            bearer: format!("Bearer {}", token),
            host_prefix: HashMap::new(),
        };

        webex
            .host_prefix
            .insert("devices".to_string(), REGISTRATION_HOST_PREFIX.to_string());

        webex
    }

    /// Get an event stream handle
    pub async fn event_stream(&self) -> Result<WebexEventStream, String> {
        /*
         * Determine the correct endpoint
         */
        let ws_url = self.get_websocket_url().await?;

        /*
         * Connect to the event stream
         */
        let url = url::Url::parse(ws_url.as_str())
            .map_err(|e| format!("Unable to parse WS URL: {}", e))?;
        debug!("Connecting to {:?}", url);

        let (mut ws_stream, _response) = connect_async(url.clone())
            .await
            .map_err(|e| format!("connecting to {}: {}", url, e))?;
        debug!("Connected to {}", url);
        self.ws_auth(&mut ws_stream).await?;

        let timeout = Duration::from_secs(20);
        Ok(WebexEventStream { ws_stream, timeout })
    }

    /// Get attachment action
    ///
    /// # Arguments
    ///
    /// * `id` - attachment ID
    ///
    /// Retrieves the attachment for the given ID.  This can be used to
    /// retrieve data from an AdaptiveCard submission
    pub async fn get_attachment_action(&self, id: &str) -> Result<types::AttachmentAction, String> {
        let rest_method = format!("attachment/actions/{}", id);
        self.api_get(rest_method.as_str()).await
    }

    /// Get a message by ID
    pub async fn get_message(&self, id: &str) -> Result<types::Message, String> {
        let rest_method = format!("messages/{}", id);
        self.api_get(rest_method.as_str()).await
    }

    /// Get available rooms
    pub async fn get_rooms(&self) -> Result<Vec<types::Room>, String> {
        let rooms_reply: Result<types::RoomsReply, _> = self.api_get("rooms").await;
        match rooms_reply {
            Err(e) => Err(format!("rooms failed: {}", e)),
            Ok(rr) => Ok(rr.items),
        }
    }

    /// Send a message to a user or room
    pub async fn send_message(
        &self,
        message: &types::MessageOut,
    ) -> Result<types::Message, String> {
        self.api_post("messages", &message).await
    }

    /******************************************************************
     * Low-level API.  These calls are chained to build various
     * high-level calls like "get_message"
     ******************************************************************/

    async fn api_get<T: DeserializeOwned>(&self, rest_method: &str) -> Result<T, String> {
        // Why do we have to say Option<String> here? Why can't we just pass in None?
        let body: Option<String> = None;
        self.rest_api("GET", rest_method, body).await
    }

    async fn api_post<T: DeserializeOwned, U: Serialize>(
        &self,
        rest_method: &str,
        body: U,
    ) -> Result<T, String> {
        self.rest_api("POST", rest_method, Some(body)).await
    }

    async fn rest_api<T: DeserializeOwned, U: Serialize>(
        &self,
        http_method: &str,
        rest_method: &str,
        body: Option<U>,
    ) -> Result<T, String> {
        match self.call_web_api_raw(http_method, rest_method, body).await {
            Ok(reply) => {
                let de: Result<T, _> = serde_json::from_str(reply.as_str());
                match de {
                    Ok(reply) => Ok(reply),
                    Err(e) => {
                        debug!("Couldn't parse reply for {} call: {}", rest_method, e);
                        debug!("Source JSON: {}", reply);
                        Err(format!("failed to parse reply: {}", e))
                    }
                }
            }
            Err(e) => Err(e),
        }
    }

    async fn call_web_api_raw<T: Serialize>(
        &self,
        http_method: &str,
        rest_method: &str,
        body: Option<T>,
    ) -> Result<String, String> {
        let default_prefix = String::from(REST_HOST_PREFIX);
        let prefix = self.host_prefix.get(rest_method).unwrap_or(&default_prefix);
        let url = format!("{}/{}", prefix, rest_method);
        debug!("Calling {} {:?}", http_method, url);
        let mut builder = Request::builder()
            .method(http_method)
            .uri(url)
            .header("Authorization", &self.bearer);
        if body.is_some() {
            builder = builder.header("Content-Type", "application/json");
        }
        let body = match body {
            Some(obj) => Body::from(serde_json::to_string(&obj).unwrap()),
            None => Body::empty(),
        };
        let req = builder.body(body).expect("request builder");
        match self.client.request(req).await {
            Ok(mut resp) => {
                let mut reply = String::new();
                while let Some(chunk) = resp.body_mut().data().await {
                    use std::str;

                    let chunk = chunk.unwrap();
                    let strchunk = str::from_utf8(&chunk).unwrap();
                    reply.push_str(&strchunk);
                }
                Ok(reply)
            }
            Err(e) => Err(format!("request failed: {}", e)),
        }
    }

    async fn get_devices(&self) -> Result<Vec<types::DeviceData>, String> {
        // https://developer.webex.com/docs/api/v1/devices
        match self.api_get::<types::DevicesReply>("devices").await {
            Ok(dd) => match dd.devices {
                Some(devices) => Ok(devices),
                None => {
                    debug!("Chaining one-time device setup from devices query");
                    self.setup_devices().await
                }
            },
            Err(e) => Err(format!("Can't decode devices reply: {}", e)),
        }
    }

    async fn setup_devices(&self) -> Result<Vec<types::DeviceData>, String> {
        let device_data = types::DeviceData {
            device_name: Some("rust-client".to_string()),
            device_type: Some("DESKTOP".to_string()),
            localized_model: Some("rust".to_string()),
            model: Some("rust".to_string()),
            name: Some("rust-spark-client".to_string()),
            system_name: "rust-spark-client".to_string(),
            system_version: Some("0.1".to_string()),
            ..Default::default()
        };

        self.api_post("devices", device_data).await
    }

    async fn get_websocket_url(&self) -> Result<String, String> {
        match self.get_devices().await?.get(0) {
            Some(device) => match &device.ws_url {
                Some(ws_url) => Ok(ws_url.clone()),
                None => Err("device missing webSocketUrl".to_string()),
            },
            None => Err("no devices returned".to_string()),
        }
    }

    async fn ws_auth(&self, ws_stream: &mut WStream) -> Result<(), String> {
        /*
         * Authenticate to the stream
         */
        let auth = types::Authorization {
            id: Uuid::new_v4().to_string(),
            _type: "authorization".to_string(),
            data: types::AuthToken {
                token: format!("Bearer {}", self.token),
            },
        };
        debug!("Authenticating to stream");
        ws_stream
            .send(Message::Text(serde_json::to_string(&auth).unwrap()))
            .await
            .map_err(|e| format!("failed to send authentication: {}", e))?;

        /*
         * The next thing back should be a pong
         */
        match ws_stream.next().await {
            Some(msg) => match msg {
                Ok(msg) => match msg {
                    Message::Pong(_) => {
                        debug!("Authentication succeeded");
                        Ok(())
                    }
                    _ => Err(format!("Received {:?} in reply to auth message", msg)),
                },
                Err(e) => Err(format!("Recieved error from websocket: {}", e)),
            },
            None => Err("Websocket closed".to_string()),
        }
    }
}

impl types::MessageOut {
    /// Generates a new outgoing message from an existing message
    ///
    /// # Arguments
    ///
    /// * `msg` - the template message
    ///
    /// Use `from_msg` to create a reply from a received message.
    pub fn from_msg(msg: &types::Message) -> Self {
        let mut new_msg: Self = Default::default();

        if msg.room_type == Some("group".to_string()) {
            new_msg.room_id = msg.room_id.clone();
        } else if let Some(person_id) = &msg.person_id {
            new_msg.to_person_id = Some(person_id.clone());
        } else {
            new_msg.to_person_email = msg.person_email.clone();
        }

        new_msg
    }
}
