#![deny(missing_docs)]
#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
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
pub use types::*;

use error::{Error, ErrorKind};

use crate::adaptive_card::AdaptiveCard;
use futures_util::{SinkExt, StreamExt};
use hyper::{body::HttpBody, client::HttpConnector, Body, Client, Request};
use hyper_tls::HttpsConnector;
use log::{debug, trace, warn};
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::HashMap, time::Duration};
use tokio::{net::TcpStream, runtime::Handle};
use tokio_tungstenite::{
    connect_async, tungstenite::Message as TMessage, MaybeTlsStream, WebSocketStream,
};

/*
 * URLs:
 *
 * https://help.webex.com/en-us/xbcr37/External-Connections-Made-by-the-Serviceability-Connector
 *
 * These apply to the central Webex Teams (Wxt) servers.  WxT also supports enterprise servers;
 * these are not supported.
 */

// Main API URL - default for any request.
const REST_HOST_PREFIX: &str = "https://api.ciscospark.com/v1";
// U2C - service discovery, used to discover other URLs (for example, the mercury URL).
const U2C_HOST_PREFIX: &str = "https://u2c.wbx2.com/u2c/api/v1";
// Default mercury URL, used when the token doesn't have permissions to list organizations.
const DEFAULT_REGISTRATION_HOST_PREFIX: &str = "https://wdm-a.wbx2.com/wdm/api/v1";

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
    pub device: DeviceData,
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
                    Some(msg) => match msg {
                        Ok(msg) => {
                            if let Some(h_msg) = self.handle_message(msg).await? {
                                return Ok(h_msg);
                            }
                            // `None` messages still reset the timeout (e.g. Ping to keep alive)
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

    async fn handle_message(&mut self, msg: TMessage) -> Result<Option<Event>, Error> {
        match msg {
            TMessage::Binary(bytes) => match std::str::from_utf8(&bytes) {
                Ok(json) => match serde_json::from_str(json) {
                    Ok(ev) => Ok(Some(ev)),
                    Err(e) => {
                        warn!("Couldn't deserialize: {:?}.  Original JSON:\n{}", e, &json);
                        Err(e.into())
                    }
                },
                Err(e) => Err(e.into()),
            },
            TMessage::Text(t) => {
                debug!("text: {}", t);
                Ok(None)
            }
            TMessage::Ping(_) => {
                trace!("Ping!");
                Ok(None)
            }
            TMessage::Close(t) => {
                debug!("close: {:?}", t);
                self.is_open = false;
                Err(ErrorKind::Closed("Web Socket Closed".to_string()).into())
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

    pub(crate) async fn auth(
        ws_stream: &mut WStream,
        token: &str,
    ) -> Result<(), Error> {
        /*
         * Authenticate to the stream
         */
        let auth = Authorization::new(token);
        debug!("Authenticating to stream");
        match ws_stream
            .send(TMessage::Text(serde_json::to_string(&auth).unwrap()))
            .await
        {
            Ok(_) => {
                /*
                 * The next thing back should be a pong
                 */
                match ws_stream.next().await {
                    Some(msg) => match msg {
                        Ok(msg) => match msg {
                            TMessage::Ping(_) | TMessage::Pong(_) => {
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
            device: DeviceData {
                device_name: Some("rust-client".to_string()),
                device_type: Some("DESKTOP".to_string()),
                localized_model: Some("rust".to_string()),
                model: Some("rust".to_string()),
                name: Some("rust-spark-client".to_string()),
                system_name: Some("rust-spark-client".to_string()),
                system_version: Some("0.1".to_string()),
                ..DeviceData::default()
            },
        };

        // Have to insert this before calling get_mercury_url() since it uses U2C for the catalog
        // request.
        webex
            .host_prefix
            .insert("limited/catalog".to_string(), U2C_HOST_PREFIX.to_string());

        let devices_url = match webex.get_mercury_url() {
            Ok(url) => {
                trace!("Fetched mercury url {}", url);
                url
            }
            Err(e) => {
                warn!("Failed to fetch devices url, falling back to default");
                debug!("Error: {}", e);
                DEFAULT_REGISTRATION_HOST_PREFIX.to_string()
            }
        };
        webex.host_prefix.insert("devices".to_string(), devices_url);

        webex
    }

    /// Get an event stream handle
    pub async fn event_stream(&self) -> Result<WebexEventStream, Error> {
        // Helper function to connect to a device
        // refactored out to make it easier to loop through all devices and also lazily create a
        // new one if needed
        async fn connect_device(
            s: &Webex,
            device: DeviceData,
        ) -> Result<WebexEventStream, Error> {
            let ws_url = match device.ws_url {
                Some(url) => url,
                None => return Err(Error::from("Device has no ws_url")),
            };
            let url = url::Url::parse(ws_url.as_str()).map_err(|_| Error::from("Failed to parse ws_url"))?;
            debug!("Connecting to {:?}", url);
            match connect_async(url.clone()).await {
                Ok((mut ws_stream, _response)) => {
                    debug!("Connected to {}", url);
                    WebexEventStream::auth(&mut ws_stream, &s.token).await?;
                    debug!("Authenticated");
                    let timeout = Duration::from_secs(20);
                    Ok(WebexEventStream {
                        ws_stream,
                        timeout,
                        is_open: true,
                    })
                }
                Err(e) => {
                    warn!("Failed to connect to {:?}: {:?}", url, e);
                    Err(ErrorKind::Tungstenite(e, "Failed to connect to ws_url".to_string()).into())
                }
            }
        }

        let mut devices: Vec<DeviceData> = match self.get_devices().await {
            Ok(d) => d,
            Err(e) => {
                warn!("Failed to get devices {}", e);
                self.setup_devices().await?;
                self.get_devices().await?
            }
        };

        devices.sort_by(|a: &DeviceData, b: &DeviceData| {
            b.modification_time
                .unwrap_or_else(chrono::Utc::now)
                .cmp(&a.modification_time.unwrap_or_else(chrono::Utc::now))
        });

        for device in devices {
            if let Ok(event_stream) = connect_device(self, device).await {
                return Ok(event_stream);
            }
        }

        // Failed to connect to any existing devices, creating new one
        if let Ok(event_stream) = connect_device(self, self.setup_devices().await?).await {
            Ok(event_stream)
        } else {
            Err(Error::from(
                "Failed to connect to any existing device and newly created device",
            ))
        }
    }

    fn get_mercury_url(&self) -> Result<String, error::Error> {
        // Steps:
        // 1. Get org id by GET /v1/organizations
        // 2. Get urls json from https://u2c.wbx2.com/u2c/api/v1/limited/catalog?orgId=[org id]
        // 3. mercury url is urls["serviceLinks"]["wdm"]
        //
        // We need to spawn a new thread because to create a new async executor, we can't be inside
        // an executor ourselves. Yes it's hacky, no there's no other way (apart from making
        // Webex::new async).

        let mut catalogs = None;
        std::thread::scope(|s| {
            let rt = Handle::current();
            catalogs = Some(
                s.spawn(move || {
                    let orgs = rt.block_on(self.get_orgs())?;
                    if orgs.len() != 1 {
                        panic!("Can only get mercury URL if account is part of exactly one org");
                    }
                    let org_id = &orgs[0].id;
                    let api_url = format!("limited/catalog?format=hostmap&orgId={}", org_id);
                    rt.block_on(self.api_get::<CatalogReply>(&api_url))
                })
                .join()
                .expect("Shouldn't panic"),
            );
        });
        Ok(catalogs
            .expect("Should have run async code")?
            .service_links
            .wdm)
    }

    /// Get list of organizations
    ///
    /// # Errors
    /// See [`Webex::get_message()`] errors.
    pub async fn get_orgs(&self) -> Result<Vec<Organization>, Error> {
        let orgs: OrganizationReply = self.api_get("organizations").await?;
        Ok(orgs.items)
    }
    /// Get attachment action
    ///
    /// # Arguments
    ///
    /// * `id` - attachment ID, a [`GlobalId`].
    ///
    /// Retrieves the attachment for the given ID.  This can be used to
    /// retrieve data from an `AdaptiveCard` submission
    ///
    /// # Errors
    /// See [`Webex::get_message()`] errors.
    pub async fn get_attachment_action(&self, id: &GlobalId) -> Result<AttachmentAction, Error> {
        debug_assert!(id.id(GlobalIdType::AttachmentAction).is_ok());
        let rest_method = format!("attachment/actions/{}", id.id_unchecked());
        self.api_get(rest_method.as_str()).await
    }

    /// Get a message by ID
    ///
    /// # Arguments
    ///
    /// * `id` - message ID, a [`GlobalId`]
    ///
    /// If you have a UUID, please use [`GlobalId::new()`].
    /// If you have an `Event`, use [`Event::get_global_id()`].
    ///
    /// # Errors
    /// Same as [`Webex::send_message()`] errors, plus an additional one below.
    /// * [`ErrorKind::Limited`] - returned on HTTP 423/429 with an optional Retry-After.
    /// * [`ErrorKind::Status`] | [`ErrorKind::StatusText`] - returned when the request results in a non-200 code.
    /// * [`ErrorKind::Json`] - returned when your input object cannot be serialized, or the return
    /// value cannot be deserialised. (If this happens, this is a library bug and should be
    /// reported.)
    /// * [`ErrorKind::UTF8`] - returned when the request returns non-UTF8 code.
    /// * (New) [`ErrorKind::IncorrectId`] - this function has been passed a ``GlobalId`` that does not
    /// correspond to a message.
    pub async fn get_message(&self, id: &GlobalId) -> Result<Message, Error> {
        debug_assert!(id.id(GlobalIdType::Message).is_ok());
        let rest_method = format!("messages/{}", id.id_unchecked());
        self.api_get(rest_method.as_str()).await
    }

    /// Delete a message by ID
    pub async fn delete_message(&self, id: &GlobalId) -> Result<(), Error> {
        debug_assert!(id.id(GlobalIdType::Message).is_ok());
        let rest_method = format!("messages/{}", id.id_unchecked());
        self.api_delete(rest_method.as_str()).await
    }

    /// Get available rooms
    pub async fn get_rooms(&self) -> Result<Vec<Room>, Error> {
        let rooms_reply: Result<RoomsReply, _> = self.api_get("rooms").await;
        match rooms_reply {
            Err(e) => Err(Error::with_chain(e, "rooms failed: ")),
            Ok(rr) => Ok(rr.items),
        }
    }

    /// Get available room
    pub async fn get_room(&self, id: &GlobalId) -> Result<Room, Error> {
        debug_assert!(id.id(GlobalIdType::Room).is_ok());
        let rest_method = format!("rooms/{}", id.id_unchecked());
        let room_reply: Result<Room, _> = self.api_get(rest_method.as_str()).await;
        match room_reply {
            Err(e) => Err(Error::with_chain(e, "room failed: ")),
            Ok(rr) => Ok(rr),
        }
    }

    /// Get information about person
    ///
    /// # Errors
    /// See `get_message`
    pub async fn get_person(&self, id: &GlobalId) -> Result<Person, Error> {
        debug_assert!(id.id(GlobalIdType::Person).is_ok());
        let rest_method = format!("people/{}", id.id_unchecked());
        let people_reply: Result<Person, _> = self.api_get(rest_method.as_str()).await;
        match people_reply {
            Err(e) => Err(Error::with_chain(e, "people failed: ")),
            Ok(pr) => Ok(pr),
        }
    }

    /// Send a message to a user or room
    ///
    /// # Arguments
    /// * `message`: [`MessageOut`] - the message to send, including one of `room_id`,
    /// `to_person_id` or `to_person_email`.
    ///
    /// # Errors
    /// Types of errors returned:
    /// * [`ErrorKind::Limited`] - returned on HTTP 423/429 with an optional Retry-After.
    /// * [`ErrorKind::Status`] | [`ErrorKind::StatusText`] - returned when the request results in a non-200 code.
    /// * [`ErrorKind::Json`] - returned when your input object cannot be serialized, or the return
    /// value cannot be deserialised. (If this happens, this is a library bug and should be
    /// reported.)
    /// * [`ErrorKind::UTF8`] - returned when the request returns non-UTF8 code.
    pub async fn send_message(&self, message: &MessageOut) -> Result<Message, Error> {
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
        let rest_method_trimmed = rest_method.split('?').next().unwrap_or(rest_method);
        let prefix = self
            .host_prefix
            .get(rest_method_trimmed)
            .unwrap_or(&default_prefix);
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

    async fn get_devices(&self) -> Result<Vec<DeviceData>, Error> {
        // https://developer.webex.com/docs/api/v1/devices
        match self.api_get::<DevicesReply>("devices").await {
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

    async fn setup_devices(&self) -> Result<DeviceData, Error> {
        self.api_post("devices", self.device.clone()).await
    }
}

impl From<&AttachmentAction> for MessageOut {
    fn from(action: &AttachmentAction) -> Self {
        MessageOut {
            room_id: action.room_id.clone(),
            ..MessageOut::default()
        }
    }
}

impl From<&Message> for MessageOut {
    fn from(msg: &Message) -> Self {
        let mut new_msg: Self = MessageOut::default();

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

impl MessageOut {
    /// Generates a new outgoing message from an existing message
    ///
    /// # Arguments
    ///
    /// * `msg` - the template message
    ///
    /// Use `from_msg` to create a reply from a received message.
    #[deprecated(since = "0.2.0", note = "Please use the from instead")]
    #[must_use]
    pub fn from_msg(msg: &Message) -> Self {
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
