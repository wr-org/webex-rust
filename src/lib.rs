#![deny(missing_docs)]
#![cfg_attr(test, deny(warnings))]
#![doc(html_root_url = "https://docs.rs/webex/0.2.0/webex/")]

//! # webex-rust
//!
//! A minimal asynchronous interface to Webex Teams, intended for (but not
//! limited to) implementing bots.
//!
//! Current functionality includes:
//!
//! - Registration with Webex APIs
//! - Monitoring an event stream
//! - Sending direct or group messages
//! - Getting room memberships
//! - Building AdaptiveCards and retrieving responses
//!
//! Not all features are fully-fleshed out, particularly the AdaptiveCard
//! support (only a few serializations exist, enough to create a form with a
//! few choices, a text box, and a submit button).
//!
//! # DISCLAIMER
//!
//! This crate is not maintained by Cisco, and not an official SDK.  The
//! author is a current developer at Cisco, but has no direct affiliation
//! with the Webex development team.

#[macro_use]
extern crate error_chain;

pub mod adaptive_card;
#[allow(missing_docs)]
pub mod error;
pub mod types;

use error::{Error, ErrorKind};

use crate::adaptive_card::AdaptiveCard;
use crate::types::Attachment;
use futures_util::{SinkExt, StreamExt};
use hyper::{body::HttpBody, client::HttpConnector, Body, Client, Request};
use hyper_tls::HttpsConnector;
use log::{debug, trace, warn};
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::HashMap, time::Duration};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
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

/// Web Socket Stream type
pub type WStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WebClient = Client<HttpsConnector<HttpConnector>, Body>;

/// Webex API Client
#[derive(Clone)]
pub struct Webex {
    client: WebClient,
    bearer: String,
    token: String,
    host_prefix: HashMap<String, String>,
    /// Webex Device Information used for device registration
    pub device: types::DeviceData,
}

/// Webex Event Stream handler
pub struct WebexEventStream {
    ws_stream: WStream,
    timeout: Duration,
    /// Signifies if WebStream is Open
    pub is_open: bool,
}

impl WebexEventStream {
    /// Get the next event from an event stream
    ///
    /// Returns an event or an error
    ///
    /// # Errors
    /// Returns an error when the underlying stream has a problem, but will
    /// continue to work on subsequent calls to `next()` - the errors can safely
    /// be ignored.
    pub async fn next(&mut self) -> Result<types::Event, Error> {
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
                        Err(tungstenite::error::Error::Protocol(e)) => {
                            // Protocol error probably requires a connection reset
                            self.is_open = false;
                            return Err(e.to_string().into());
                        }
                        Err(e) => return Err(e.to_string().into()),
                    },
                    None => continue,
                },
            }
        }
    }

    async fn handle_message(&mut self, msg: Message) -> Result<Option<types::Event>, Error> {
        match msg {
            Message::Binary(bytes) => match std::str::from_utf8(&bytes) {
                Ok(json) => match serde_json::from_str(json) {
                    Ok(ev) => Ok(Some(ev)),
                    Err(e) => {
                        warn!("Couldn't deserialize: {:?}.  Original JSON:\n{}", e, &json);
                        Err(e.into())
                    }
                },
                Err(e) => Err(e.into()),
            },
            Message::Text(t) => {
                debug!("text: {}", t);
                Ok(None)
            }
            Message::Ping(_) => {
                trace!("Ping!");
                Ok(None)
            }
            Message::Close(t) => {
                debug!("close: {:?}", t);
                self.is_open = false;
                Err(ErrorKind::Closed("Web Socket Closed".to_string()).into())
            }
            Message::Pong(_) => {
                debug!("Pong!");
                Ok(None)
            }
            Message::Frame(_) => {
                debug!("Frame");
                Ok(None)
            }
        }
    }
}

impl Webex {
    /// Constructs a new Webex Teams context from a token
    /// Tokens can be obtained when creating a bot, see <https://developer.webex.com/my-apps> for
    /// more information and to create your own Webex bots.
    #[must_use]
    pub fn new(token: &str) -> Self {
        let https = HttpsConnector::new();
        let client = Client::builder().build::<_, hyper::Body>(https);

        let mut webex = Webex {
            client,
            token: token.to_string(),
            bearer: format!("Bearer {}", token),
            host_prefix: HashMap::new(),
            device: types::DeviceData {
                device_name: Some("rust-client".to_string()),
                device_type: Some("DESKTOP".to_string()),
                localized_model: Some("rust".to_string()),
                model: Some("rust".to_string()),
                name: Some("rust-spark-client".to_string()),
                system_name: Some("rust-spark-client".to_string()),
                system_version: Some("0.1".to_string()),
                ..Default::default()
            },
        };

        webex
            .host_prefix
            .insert("devices".to_string(), REGISTRATION_HOST_PREFIX.to_string());

        webex
    }

    /// Get an event stream handle
    pub async fn event_stream(&self) -> Result<WebexEventStream, Error> {
        let mut devices: Vec<types::DeviceData> = match self.get_devices().await {
            Ok(d) => d,
            Err(e) => {
                warn!("Failed to get devices {}", e);
                if let Err(e) = self.setup_devices().await {
                    return Err(e);
                };
                match self.get_devices().await {
                    Ok(d) => d,
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
        };

        devices.sort_by(|a: &types::DeviceData, b: &types::DeviceData| {
            b.modification_time
                .unwrap_or_else(chrono::Utc::now)
                .cmp(&a.modification_time.unwrap_or_else(chrono::Utc::now))
        });

        for device in devices {
            if let Some(ws_url) = device.ws_url {
                let url = match url::Url::parse(ws_url.as_str()) {
                    Ok(u) => u,
                    Err(e) => {
                        warn!("Failed to parse {:?}", e);
                        continue;
                    }
                };
                debug!("Connecting to {:?}", url);
                match connect_async(url.clone()).await {
                    Ok((mut ws_stream, _response)) => {
                        debug!("Connected to {}", url);
                        self.ws_auth(&mut ws_stream).await?;
                        debug!("Authenticated");
                        let timeout = Duration::from_secs(20);
                        return Ok(WebexEventStream {
                            ws_stream,
                            timeout,
                            is_open: true,
                        });
                    }
                    Err(e) => {
                        warn!("Failed to connect to {:?}: {:?}", url, e);
                        continue;
                    }
                };
            }
        }

        // Failed to connect to any existing devices, creating new one
        let ws_url = match self.setup_devices().await {
            Ok(d) => match d.ws_url {
                Some(url) => url.clone(),
                None => return Err("Registered device has no ws url".into()),
            },
            Err(e) => {
                return Err(format!("Failed to setup device: {}", e).into());
            }
        };

        let url = url::Url::parse(ws_url.as_str())
            .map_err(|e| Into::<Error>::into(format!("Unable to parse WS URL {}", e)))?;
        debug!("Connecting to #2 {:?}", url);

        match connect_async(url.clone()).await {
            Ok((mut ws_stream, _response)) => {
                debug!("Connected to {}", url);
                self.ws_auth(&mut ws_stream).await?;

                let timeout = Duration::from_secs(20);
                Ok(WebexEventStream {
                    ws_stream,
                    timeout,
                    is_open: true,
                })
            }
            Err(e) => Err(Into::<Error>::into(format!("connecting to {}, {}", url, e))),
        }
    }

    /// Get attachment action
    ///
    /// # Arguments
    ///
    /// * `id` - attachment ID
    ///
    /// Retrieves the attachment for the given ID.  This can be used to
    /// retrieve data from an `AdaptiveCard` submission
    pub async fn get_attachment_action(&self, id: &str) -> Result<types::AttachmentAction, Error> {
        let rest_method = match Uuid::parse_str(id) {
            Ok(_) => format!(
                "attachment/actions/{}",
                base64::encode(format!("ciscospark://us/ATTACHMENT_ACTION/{}", id))
            ),
            Err(_) => {
                format!("attachment/actions/{}", id)
            }
        };
        self.api_get(rest_method.as_str()).await
    }

    /// Get a message by ID
    ///
    /// # Arguments
    ///
    /// * `id` - a [`types::MessageId`]
    ///
    /// If you have a UUID, please use [`types::MessageId::new()`] or [`types::MessageId::from()`].
    /// If you have an `Activity`, use [`types::Activity::get_message_id()`].
    ///
    /// # Errors
    /// See [Webex::send_message()] errors.
    pub async fn get_message(&self, id: &types::MessageId) -> Result<types::Message, Error> {
        //self.get_message_with_cluster(id, "us").await
        let rest_method = format!("messages/{}", id.id());
        self.api_get(rest_method.as_str()).await
    }

    /// Delete a message by ID
    pub async fn delete_message(&self, id: &str) -> Result<(), Error> {
        let rest_method = format!("messages/{}", id);
        self.api_delete(rest_method.as_str()).await
    }

    /// Get available rooms
    pub async fn get_rooms(&self) -> Result<Vec<types::Room>, Error> {
        let rooms_reply: Result<types::RoomsReply, _> = self.api_get("rooms").await;
        match rooms_reply {
            Err(e) => Err(Error::with_chain(e, "rooms failed: ")),
            Ok(rr) => Ok(rr.items),
        }
    }

    /// Get available room
    pub async fn get_room(&self, id: &str) -> Result<types::Room, Error> {
        let rest_method = match Uuid::parse_str(id) {
            Ok(_) => format!(
                "rooms/{}",
                base64::encode(format!("ciscospark://us/ROOM/{}", id))
            ),
            Err(_) => {
                format!("rooms/{}", id)
            }
        };
        let room_reply: Result<types::Room, _> = self.api_get(rest_method.as_str()).await;
        match room_reply {
            Err(e) => Err(Error::with_chain(e, "room failed: ")),
            Ok(rr) => Ok(rr),
        }
    }

    /// Get information about person
    ///
    /// # Errors
    /// See `send_message`
    pub async fn get_person(&self, id: &str) -> Result<types::Person, Error> {
        let rest_method = match Uuid::parse_str(id) {
            Ok(_) => format!(
                "people/{}",
                base64::encode(format!("ciscospark://us/PEOPLE/{}", id))
            ),
            Err(_) => {
                format!("people/{}", id)
            }
        };
        let people_reply: Result<types::Person, _> = self.api_get(rest_method.as_str()).await;
        match people_reply {
            Err(e) => Err(Error::with_chain(e, "people failed: ")),
            Ok(pr) => Ok(pr),
        }
    }

    /// Send a message to a user or room
    ///
    /// # Errors
    /// Types of errors returned:
    /// * [`ErrorKind::Limited`] - returned on HTTP 423/429 with an optional Retry-After.
    /// * [`ErrorKind::Status`] | [`ErrorKind::StatusText`] - returned when the request results in a non-200 code.
    /// * [`ErrorKind::Json`] - returned when your input object cannot be serialized, or the return
    /// value cannot be deserialised. (If this happens, this is a library bug and should be
    /// reported.)
    /// * [`ErrorKind::UTF8`] - returned when the requests returns non-UTF8 code.
    pub async fn send_message(&self, message: &types::MessageOut) -> Result<types::Message, Error> {
        self.api_post("messages", &message).await
    }

    /******************************************************************
     * Low-level API.  These calls are chained to build various
     * high-level calls like "get_message"
     ******************************************************************/

    async fn api_get<T: DeserializeOwned>(&self, rest_method: &str) -> Result<T, Error> {
        let body: Option<String> = None;
        self.rest_api("GET", rest_method, body).await
    }

    async fn api_delete(&self, rest_method: &str) -> Result<(), Error> {
        let body: Option<String> = None;
        self.rest_api("DELETE", rest_method, body).await
    }

    async fn api_post<T: DeserializeOwned, U: Serialize>(
        &self,
        rest_method: &str,
        body: U,
    ) -> Result<T, Error> {
        self.rest_api("POST", rest_method, Some(body)).await
    }

    async fn rest_api<T: DeserializeOwned, U: Serialize>(
        &self,
        http_method: &str,
        rest_method: &str,
        body: Option<U>,
    ) -> Result<T, Error> {
        match self.call_web_api_raw(http_method, rest_method, body).await {
            Ok(reply) => {
                let de: Result<T, _> = serde_json::from_str(reply.as_str());
                match de {
                    Ok(reply) => Ok(reply),
                    Err(e) => {
                        debug!("Couldn't parse reply for {} call: {}", rest_method, e);
                        debug!("Source JSON: {}", reply);
                        Err(Error::with_chain(e, "failed to parse reply"))
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
    ) -> Result<String, Error> {
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
            Some(obj) => Body::from(serde_json::to_string(&obj)?),
            None => Body::empty(),
        };
        let req = builder.body(body).expect("request builder");
        match self.client.request(req).await {
            Ok(mut resp) => {
                let mut reply = String::new();
                while let Some(chunk) = resp.body_mut().data().await {
                    use std::str;

                    let chunk = chunk?;
                    let strchunk = str::from_utf8(&chunk)?;
                    reply.push_str(strchunk);
                }
                match resp.status() {
                    hyper::StatusCode::LOCKED | hyper::StatusCode::TOO_MANY_REQUESTS => {
                        warn!("Limited");
                        let retry_after = resp
                            .headers()
                            .get("Retry-After")
                            .and_then(|s| s.to_str().ok())
                            .and_then(|t| t.parse::<i64>().ok());
                        Err(ErrorKind::Limited(resp.status(), retry_after).into())
                    }
                    status if !status.is_success() => {
                        Err(ErrorKind::StatusText(resp.status(), reply).into())
                    }
                    _ => Ok(reply),
                }
            }
            Err(e) => Err(Error::with_chain(e, "request failed")),
        }
    }

    async fn get_devices(&self) -> Result<Vec<types::DeviceData>, Error> {
        // https://developer.webex.com/docs/api/v1/devices
        match self.api_get::<types::DevicesReply>("devices").await {
            Ok(dd) => match dd.devices {
                Some(devices) => Ok(devices),
                None => {
                    debug!("Chaining one-time device setup from devices query");
                    match self.setup_devices().await {
                        Ok(device) => Ok(vec![device]),
                        Err(e) => Err(e),
                    }
                }
            },
            Err(e) => match e {
                Error(ErrorKind::Status(s) | ErrorKind::StatusText(s, _), _) => {
                    if s == hyper::StatusCode::NOT_FOUND {
                        debug!("No devices found, creating new one");
                        match self.setup_devices().await {
                            Ok(device) => Ok(vec![device]),
                            Err(e) => Err(e),
                        }
                    } else {
                        Err(Error::with_chain(e, "Can't decode devices reply"))
                    }
                }
                Error(ErrorKind::Limited(_, t), _) => Err(Error::with_chain(
                    e,
                    format!("We are hitting the API limit, retry after: {:?}", t),
                )),
                _ => Err(format!("Can't decode devices reply: {}", e).into()),
            },
        }
    }

    async fn setup_devices(&self) -> Result<types::DeviceData, Error> {
        self.api_post("devices", self.device.clone()).await
    }

    async fn ws_auth(
        &self,
        ws_stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    ) -> Result<(), Error> {
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
        match ws_stream
            .send(Message::Text(serde_json::to_string(&auth).unwrap()))
            .await
        {
            Ok(_) => {
                /*
                 * The next thing back should be a pong
                 */
                match ws_stream.next().await {
                    Some(msg) => match msg {
                        Ok(msg) => match msg {
                            Message::Ping(_) | Message::Pong(_) => {
                                debug!("Authentication succeeded");
                                Ok(())
                            }
                            _ => Err(format!("Received {:?} in reply to auth message", msg).into()),
                        },
                        Err(e) => Err(format!("Received error from websocket: {}", e).into()),
                    },
                    None => Err("Websocket closed".to_string().into()),
                }
            }
            Err(e) => {
                Err(ErrorKind::Tungstenite(e, "failed to send authentication".to_string()).into())
            }
        }
    }
}

impl From<&types::AttachmentAction> for types::MessageOut {
    fn from(action: &types::AttachmentAction) -> Self {
        types::MessageOut {
            room_id: action.room_id.clone(),
            ..types::MessageOut::default()
        }
    }
}

impl From<&types::Message> for types::MessageOut {
    fn from(msg: &types::Message) -> Self {
        let mut new_msg: Self = types::MessageOut::default();

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

impl types::MessageOut {
    /// Generates a new outgoing message from an existing message
    ///
    /// # Arguments
    ///
    /// * `msg` - the template message
    ///
    /// Use `from_msg` to create a reply from a received message.
    #[deprecated(since = "0.2.0", note = "Please use the from instead")]
    #[must_use]
    pub fn from_msg(msg: &types::Message) -> Self {
        Self::from(msg)
    }

    /// Add attachment to an existing message
    ///
    /// # Arguments
    ///
    /// * `card` - Adaptive Card to attach
    pub fn add_attachment(&mut self, card: AdaptiveCard) -> &Self {
        self.attachments = Some(vec![Attachment {
            content_type: "application/vnd.microsoft.card.adaptive".to_string(),
            content: card,
        }]);
        self
    }
}
