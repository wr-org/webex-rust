#![deny(missing_docs)]
#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::option_if_let_else)]
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
extern crate lazy_static;

pub mod adaptive_card;
#[allow(missing_docs)]
pub mod error;
pub mod types;
pub use types::*;
pub mod auth;

use error::{Error, ErrorKind, ResultExt};

use crate::adaptive_card::AdaptiveCard;
use crate::types::Attachment;
use base64::{engine::general_purpose as bas64enc, Engine as _};
use futures::{future::try_join_all, try_join};
use futures_util::{SinkExt, StreamExt};
use hyper::{body::HttpBody, client::HttpConnector, Body, Client, Request};
use hyper_tls::HttpsConnector;
use log::{debug, trace, warn};
use serde::de::DeserializeOwned;
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{self, Hasher},
    sync::Mutex,
    time::Duration,
};
use tokio::net::TcpStream;
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

const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Web Socket Stream type
pub type WStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WebClient = Client<HttpsConnector<HttpConnector>, Body>;

/// Webex API Client
#[derive(Clone)]
#[must_use]
pub struct Webex {
    id: u64,
    client: RestClient,
    token: String,
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

            use tokio_tungstenite::tungstenite::Error as TErr;
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
                    None => continue,
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
                            return Err(ErrorKind::Tungstenite(
                                e,
                                "Error getting next_result".into(),
                            )
                            .into())
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

    pub(crate) async fn auth(ws_stream: &mut WStream, token: &str) -> Result<(), Error> {
        /*
         * Authenticate to the stream
         */
        let auth = types::Authorization::new(token);
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
                            _ => Err(format!("Received {msg:?} in reply to auth message").into()),
                        },
                        Err(e) => Err(format!("Received error from websocket: {e}").into()),
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

enum Authorization<'a> {
    None,
    Bearer(&'a str),
    Basic {
        username: &'a str,
        password: &'a str,
    },
}

/// Implements low level REST requests to be used internally by the library
#[derive(Clone)]
struct RestClient {
    host_prefix: HashMap<String, String>,
    web_client: WebClient,
}

struct RequestBody<T>
where
    Body: From<T>,
{
    media_type: &'static str,
    content: T,
}

impl RestClient {
    /// Creates a new `RestClient`
    pub fn new() -> Self {
        let https = HttpsConnector::new();
        let web_client = Client::builder().build::<_, hyper::Body>(https);
        Self {
            host_prefix: HashMap::new(),
            web_client,
        }
    }

    /******************************************************************
     * Low-level API.  These calls are chained to build various
     * high-level calls like "get_message"
     ******************************************************************/

    async fn api_get<'a, T: DeserializeOwned>(
        &self,
        rest_method: &str,
        auth: Authorization<'a>,
    ) -> Result<T, Error> {
        let body: Option<RequestBody<String>> = None;
        self.rest_api("GET", rest_method, body, auth).await
    }

    async fn api_delete<'a>(
        &self,
        rest_method: &str,
        auth: Authorization<'a>,
    ) -> Result<(), Error> {
        let body: Option<RequestBody<String>> = None;
        self.rest_api("DELETE", rest_method, body, auth).await
    }

    async fn api_post<'a, T: DeserializeOwned, U>(
        &self,
        rest_method: &str,
        body: RequestBody<U>,
        auth: Authorization<'a>,
    ) -> Result<T, Error>
    where
        Body: From<U>,
    {
        self.rest_api("POST", rest_method, Some(body), auth).await
    }

    async fn rest_api<'a, T: DeserializeOwned, U>(
        &self,
        http_method: &str,
        rest_method: &str,
        body: Option<RequestBody<U>>,
        auth: Authorization<'a>,
    ) -> Result<T, Error>
    where
        Body: From<U>,
    {
        let reply = self
            .call_web_api_raw(http_method, rest_method, body, auth)
            .await?;
        let mut reply_str = reply.as_str();
        if reply_str.is_empty() {
            reply_str = "null";
        }
        serde_json::from_str(reply_str).map_err(|e| {
            debug!("Couldn't parse reply for {} call: {}", rest_method, e);
            debug!("Source JSON: `{}`", reply_str);
            Error::with_chain(e, "failed to parse reply")
        })
    }

    async fn call_web_api_raw<'a, T>(
        &self,
        http_method: &str,
        rest_method: &str,
        body: Option<RequestBody<T>>,
        auth: Authorization<'a>,
    ) -> Result<String, Error>
    where
        Body: From<T>,
    {
        let default_prefix = String::from(REST_HOST_PREFIX);
        let rest_method_trimmed = rest_method.split('?').next().unwrap_or(rest_method);
        let prefix = self
            .host_prefix
            .get(rest_method_trimmed)
            .unwrap_or(&default_prefix);
        let url = format!("{prefix}/{rest_method}");
        debug!("Calling {} {:?}", http_method, url);
        let mut builder = match auth {
            Authorization::None => Request::builder(),
            Authorization::Basic { username, password } => {
                let encoded = bas64enc::STANDARD_NO_PAD.encode(format!("{username}:{password}"));
                Request::builder().header("Authorization", format!("Basic {encoded}"))
            }
            Authorization::Bearer(token) => {
                Request::builder().header("Authorization", format!("Bearer {token}"))
            }
        }
        .method(http_method)
        .uri(url);
        let body = match body {
            None => Body::empty(),
            Some(request_body) => {
                builder = builder.header("Content-Type", request_body.media_type);
                Body::from(request_body.content)
            }
        };
        let req = builder.body(body).expect("request builder");
        match self.web_client.request(req).await {
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
                        let retry_after = resp
                            .headers()
                            .get("Retry-After")
                            .and_then(|s| s.to_str().ok())
                            .and_then(|t| t.parse::<i64>().ok());
                        warn!(
                            "Limited calling {} {}/{}",
                            http_method, prefix, rest_method_trimmed
                        );
                        debug!("Retry-After: {:?}", retry_after);
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
}

impl Webex {
    /// Constructs a new Webex Teams context from a token
    /// Tokens can be obtained when creating a bot, see <https://developer.webex.com/my-apps> for
    /// more information and to create your own Webex bots.
    pub async fn new(token: &str) -> Self {
        let https = HttpsConnector::new();
        let web_client = Client::builder().build::<_, hyper::Body>(https);
        let mut client: RestClient = RestClient {
            host_prefix: HashMap::new(),
            web_client,
        };

        let mut hasher = DefaultHasher::new();
        hash::Hash::hash_slice(token.as_bytes(), &mut hasher);
        let id = hasher.finish();

        // Have to insert this before calling get_mercury_url() since it uses U2C for the catalog
        // request.
        client
            .host_prefix
            .insert("limited/catalog".to_string(), U2C_HOST_PREFIX.to_string());

        let mut webex = Self {
            id,
            client,
            token: token.to_string(),
            device: DeviceData {
                device_name: Some("rust-client".to_string()),
                device_type: Some("DESKTOP".to_string()),
                localized_model: Some("rust".to_string()),
                model: Some(format!("rust-v{CRATE_VERSION}")),
                name: Some("rust-spark-client".to_string()),
                system_name: Some("rust-spark-client".to_string()),
                system_version: Some(CRATE_VERSION.to_string()),
                ..DeviceData::default()
            },
        };

        let devices_url = match webex.get_mercury_url().await {
            Ok(url) => {
                trace!("Fetched mercury url {}", url);
                url
            }
            Err(e) => {
                debug!("Failed to fetch devices url, falling back to default");
                debug!("Error: {:?}", e);
                DEFAULT_REGISTRATION_HOST_PREFIX.to_string()
            }
        };
        webex
            .client
            .host_prefix
            .insert("devices".to_string(), devices_url);

        webex
    }

    /// Get an event stream handle
    pub async fn event_stream(&self) -> Result<WebexEventStream, Error> {
        // Helper function to connect to a device
        // refactored out to make it easier to loop through all devices and also lazily create a
        // new one if needed
        async fn connect_device(s: &Webex, device: DeviceData) -> Result<WebexEventStream, Error> {
            let ws_url = match device.ws_url {
                Some(url) => url,
                None => return Err("Device has no ws_url".into()),
            };
            let url = url::Url::parse(ws_url.as_str())
                .map_err(|_| Error::from("Failed to parse ws_url"))?;
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

        // get_devices automatically tries to set up devices if the get fails.
        let mut devices: Vec<DeviceData> = self.get_devices().await?;

        // Sort devices in descending order by modification time, meaning latest created device
        // first.
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
        connect_device(self, self.setup_devices().await?).await
    }

    async fn get_mercury_url(&self) -> Result<String, Option<error::Error>> {
        // Bit of a hacky workaround, error::Error does not implement clone
        // TODO: this can be fixed by returning a Result<String, &error::Error>
        lazy_static::lazy_static! {
            static ref MERCURY_CACHE: Mutex<HashMap<u64, Result<String, ()>>> = Mutex::new(HashMap::new());
        }
        if let Ok(Some(result)) = MERCURY_CACHE
            .lock()
            .map(|cache| cache.get(&self.id).map(Clone::clone))
        {
            trace!("Found mercury URL in cache!");
            return result.map_err(|_| None);
        }

        let mercury_url = self.get_mercury_url_uncached().await;

        if let Ok(mut cache) = MERCURY_CACHE.lock() {
            let result = mercury_url.as_ref().map_or(Err(()), |url| Ok(url.clone()));
            trace!("Saving mercury url to cache: {}=>{:?}", self.id, &result);
            cache.insert(self.id, result);
        }

        mercury_url.map_err(Some)
    }

    async fn get_mercury_url_uncached(&self) -> Result<String, error::Error> {
        // Steps:
        // 1. Get org id by GET /v1/organizations
        // 2. Get urls json from https://u2c.wbx2.com/u2c/api/v1/limited/catalog?orgId=[org id]
        // 3. mercury url is urls["serviceLinks"]["wdm"]
        //
        // 4. Add caching because this doesn't change, and it can be slow

        let orgs = self.list::<Organization>().await?;
        if orgs.is_empty() {
            return Err("Can't get mercury URL with no orgs".into());
        }
        let org_id = &orgs[0].id;
        let api_url = format!("limited/catalog?format=hostmap&orgId={org_id}");
        let catalogs = self
            .client
            .api_get::<CatalogReply>(&api_url, Authorization::Bearer(&self.token))
            .await?;
        let mercury_url = catalogs.service_links.wdm;

        Ok(mercury_url)
    }

    /// Get list of organizations
    #[deprecated(
        since = "0.6.3",
        note = "Please use `webex::list::<Organization>()` instead"
    )]
    pub async fn get_orgs(&self) -> Result<Vec<Organization>, Error> {
        self.list().await
    }
    /// Get attachment action
    /// Retrieves the attachment for the given ID.  This can be used to
    /// retrieve data from an `AdaptiveCard` submission
    #[deprecated(
        since = "0.6.3",
        note = "Please use `webex::get::<AttachmentAction>(id)` instead"
    )]
    pub async fn get_attachment_action(&self, id: &GlobalId) -> Result<AttachmentAction, Error> {
        self.get(id).await
    }

    /// Get a message by ID
    #[deprecated(
        since = "0.6.3",
        note = "Please use `webex::get::<Message>(id)` instead"
    )]
    pub async fn get_message(&self, id: &GlobalId) -> Result<Message, Error> {
        self.get(id).await
    }

    /// Delete a message by ID
    #[deprecated(
        since = "0.6.3",
        note = "Please use `webex::delete::<Message>(id)` instead"
    )]
    pub async fn delete_message(&self, id: &GlobalId) -> Result<(), Error> {
        self.delete::<Message>(id).await
    }

    /// Get available rooms
    #[deprecated(since = "0.6.3", note = "Please use `webex::list::<Room>()` instead")]
    pub async fn get_rooms(&self) -> Result<Vec<Room>, Error> {
        self.list().await
    }

    /// Get all rooms from all organizations that the client belongs to.
    /// Will be slow as does multiple API calls (one to get teamless rooms, one to get teams, then
    /// one per team).
    pub async fn get_all_rooms(&self) -> Result<Vec<Room>, Error> {
        let (mut all_rooms, teams) = try_join!(self.list(), self.list::<Team>())?;
        let team_endpoints: Vec<_> = teams
            .into_iter()
            .map(|team| format!("{}/?teamId={}", Room::API_ENDPOINT, team.id))
            .collect();
        let futures: Vec<_> = team_endpoints
            .iter()
            .map(|endpoint| {
                self.client
                    .api_get::<ListResult<Room>>(endpoint, Authorization::Bearer(&self.token))
            })
            .collect();
        let teams_rooms = try_join_all(futures).await?;
        for room in teams_rooms {
            all_rooms.extend(room.items);
        }
        Ok(all_rooms)
    }

    /// Get available room
    #[deprecated(since = "0.6.3", note = "Please use `webex::get::<Room>(id)` instead")]
    pub async fn get_room(&self, id: &GlobalId) -> Result<Room, Error> {
        self.get(id).await
    }

    /// Get information about person
    #[deprecated(
        since = "0.6.3",
        note = "Please use `webex::get::<Person>(id)` instead"
    )]
    pub async fn get_person(&self, id: &GlobalId) -> Result<Person, Error> {
        self.get(id).await
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
        self.client
            .api_post(
                "messages",
                RequestBody {
                    media_type: "application/json",
                    content: serde_json::to_string(&message)?,
                },
                Authorization::Bearer(&self.token),
            )
            .await
    }

    /// Get a resource from an ID
    /// # Errors
    /// * [`ErrorKind::Limited`] - returned on HTTP 423/429 with an optional Retry-After.
    /// * [`ErrorKind::Status`] | [`ErrorKind::StatusText`] - returned when the request results in a non-200 code.
    /// * [`ErrorKind::Json`] - returned when your input object cannot be serialized, or the return
    /// value cannot be deserialised. (If this happens, this is a library bug and should be
    /// reported.)
    /// * [`ErrorKind::UTF8`] - returned when the request returns non-UTF8 code.
    pub async fn get<T: Gettable + DeserializeOwned>(&self, id: &GlobalId) -> Result<T, Error> {
        let rest_method = format!("{}/{}", T::API_ENDPOINT, id.id());
        self.client
            .api_get::<T>(rest_method.as_str(), Authorization::Bearer(&self.token))
            .await
            .chain_err(|| {
                format!(
                    "Failed to get {} with id {:?}",
                    std::any::type_name::<T>(),
                    id
                )
            })
    }

    /// Delete a resource from an ID
    pub async fn delete<T: Gettable + DeserializeOwned>(&self, id: &GlobalId) -> Result<(), Error> {
        let rest_method = format!("{}/{}", T::API_ENDPOINT, id.id());
        self.client
            .api_delete(rest_method.as_str(), Authorization::Bearer(&self.token))
            .await
            .chain_err(|| {
                format!(
                    "Failed to delete {} with id {:?}",
                    std::any::type_name::<T>(),
                    id
                )
            })
    }

    /// List resources of a type
    pub async fn list<T: Gettable + DeserializeOwned>(&self) -> Result<Vec<T>, Error> {
        self.client
            .api_get::<ListResult<T>>(T::API_ENDPOINT, Authorization::Bearer(&self.token))
            .await
            .map(|result| result.items)
            .chain_err(|| format!("Failed to list {}", std::any::type_name::<T>()))
    }

    async fn get_devices(&self) -> Result<Vec<DeviceData>, Error> {
        match self
            .client
            .api_get::<DevicesReply>("devices", Authorization::Bearer(&self.token))
            .await
        {
            #[rustfmt::skip]
            Ok(DevicesReply { devices: Some(devices), .. }) => Ok(devices),
            Ok(DevicesReply { devices: None, .. }) => {
                debug!("Chaining one-time device setup from devices query");
                self.setup_devices().await.map(|device| vec![device])
            }
            Err(e) => match e {
                Error(ErrorKind::Status(s) | ErrorKind::StatusText(s, _), _) => {
                    if s == hyper::StatusCode::NOT_FOUND {
                        debug!("No devices found, creating new one");
                        self.setup_devices().await.map(|device| vec![device])
                    } else {
                        Err(Error::with_chain(e, "Can't decode devices reply"))
                    }
                }
                Error(ErrorKind::Limited(_, _), _) => Err(e),
                _ => Err(format!("Can't decode devices reply: {e}").into()),
            },
        }
    }

    async fn setup_devices(&self) -> Result<DeviceData, Error> {
        self.client
            .api_post(
                "devices",
                RequestBody {
                    media_type: "application/json",
                    content: serde_json::to_string(&self.device)?,
                },
                Authorization::Bearer(&self.token),
            )
            .await
    }
}

impl From<&AttachmentAction> for MessageOut {
    fn from(action: &AttachmentAction) -> Self {
        Self {
            room_id: action.room_id.clone(),
            ..Self::default()
        }
    }
}

impl From<&Message> for MessageOut {
    fn from(msg: &Message) -> Self {
        let mut new_msg = Self::default();

        if msg.room_type == Some(RoomType::Group) {
            new_msg.room_id = msg.room_id.clone();
        } else if let Some(person_id) = &msg.person_id {
            new_msg.to_person_id = Some(person_id.clone());
        } else {
            new_msg.to_person_email = msg.person_email.clone();
        }

        new_msg
    }
}

impl Message {
    /// Reply to a message.
    /// Posts the reply in the same chain as the replied-to message.
    /// Contrast with [`MessageOut::from()`] which only replies in the same room.
    #[must_use]
    pub fn reply(&self) -> MessageOut {
        let mut msg = MessageOut::from(self);
        msg.parent_id = self
            .parent_id
            .as_deref()
            .or(self.id.as_deref())
            .map(ToOwned::to_owned);
        msg
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
