#![deny(missing_docs)]
#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
// clippy::use_self fixed in https://github.com/rust-lang/rust-clippy/pull/9454
// TODO: remove this when clippy bug fixed in stable
#![allow(clippy::use_self)]
// should support this in the future - would be nice if all futures were send
#![allow(clippy::future_not_send)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::option_if_let_else)]
#![cfg_attr(test, deny(warnings))]
#![doc(html_root_url = "https://docs.rs/webex/latest/webex/")]

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
//! - Building `AdaptiveCards` and retrieving responses
//!
//! Not all features are fully-fleshed out, particularly the `AdaptiveCard`
//! support (only a few serializations exist, enough to create a form with a
//! few choices, a text box, and a submit button).
//!
//! # DISCLAIMER
//!
//! This crate is not maintained by Cisco, and not an official SDK.  The
//! author is a current developer at Cisco, but has no direct affiliation
//! with the Webex development team.

extern crate lazy_static;

pub mod adaptive_card;
#[allow(missing_docs)]
pub mod error;
pub mod types;
pub use types::*;
pub mod auth;

use error::Error;

use crate::adaptive_card::AdaptiveCard;
use futures::{future::try_join_all, try_join};
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, trace, warn};
use reqwest::StatusCode;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{self, Hasher},
    sync::Mutex,
    time::Duration,
};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Error as TErr, Message as TMessage},
    MaybeTlsStream, WebSocketStream,
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

// Qualify webex devices created by this crate
const DEFAULT_DEVICE_NAME: &str = "rust-client";
const DEVICE_SYSTEM_NAME: &str = "rust-spark-client";

/// Web Socket Stream type
pub type WStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

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
    /// Signifies if `WebStream` is Open
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
                            return Err(Error::Tungstenite(e, "Error getting next_result".into()))
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
            Ok(()) => {
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
            Err(e) => Err(Error::Tungstenite(
                e,
                "failed to send authentication".to_string(),
            )),
        }
    }
}

enum AuthorizationType<'a> {
    None,
    Bearer(&'a str),
    Basic {
        username: &'a str,
        password: &'a str,
    },
}

enum Body<T: Serialize> {
    Json(T),
    UrlEncoded(T),
}

const BODY_NONE: Option<Body<()>> = None;

/// Implements low level REST requests to be used internally by the library
#[derive(Clone)]
struct RestClient {
    host_prefix: HashMap<String, String>,
    web_client: reqwest::Client,
}

impl RestClient {
    /// Creates a new `RestClient`
    pub fn new() -> Self {
        Self {
            host_prefix: HashMap::new(),
            web_client: reqwest::Client::new(),
        }
    }

    /******************************************************************
     * Low-level API.  These calls are chained to build various
     * high-level calls like "get_message"
     ******************************************************************/

    async fn api_get<'a, T: DeserializeOwned>(
        &self,
        rest_method: &str,
        params: Option<impl Serialize>,
        auth: AuthorizationType<'a>,
    ) -> Result<T, Error> {
        self.rest_api(reqwest::Method::GET, rest_method, auth, params, BODY_NONE)
            .await
    }

    async fn api_delete<'a>(
        &self,
        rest_method: &str,
        params: Option<impl Serialize>,
        auth: AuthorizationType<'a>,
    ) -> Result<(), Error> {
        self.rest_api(
            reqwest::Method::DELETE,
            rest_method,
            auth,
            params,
            BODY_NONE,
        )
        .await
    }

    async fn api_post<'a, T: DeserializeOwned>(
        &self,
        rest_method: &str,
        body: impl Serialize,
        params: Option<impl Serialize>,
        auth: AuthorizationType<'a>,
    ) -> Result<T, Error>
where {
        self.rest_api(
            reqwest::Method::POST,
            rest_method,
            auth,
            params,
            Some(Body::Json(body)),
        )
        .await
    }

    async fn api_post_form_urlencoded<'a, T: DeserializeOwned>(
        &self,
        rest_method: &str,
        body: impl Serialize,
        params: Option<impl Serialize>,
        auth: AuthorizationType<'a>,
    ) -> Result<T, Error> {
        self.rest_api(
            reqwest::Method::POST,
            rest_method,
            auth,
            params,
            Some(Body::UrlEncoded(body)),
        )
        .await
    }

    async fn api_put<'a, T: DeserializeOwned>(
        &self,
        rest_method: &str,
        body: impl Serialize,
        params: Option<impl Serialize>,
        auth: AuthorizationType<'a>,
    ) -> Result<T, Error> {
        self.rest_api(
            reqwest::Method::PUT,
            rest_method,
            auth,
            params,
            Some(Body::Json(body)),
        )
        .await
    }

    async fn rest_api<T: DeserializeOwned>(
        &self,
        http_method: reqwest::Method,
        url: &str,
        auth: AuthorizationType<'_>,
        params: Option<impl Serialize>,
        body: Option<Body<impl Serialize>>,
    ) -> Result<T, Error> {
        let url_trimmed = url.split('?').next().unwrap_or(url);
        let prefix = self
            .host_prefix
            .get(url_trimmed)
            .map_or(REST_HOST_PREFIX, String::as_str);
        let url = format!("{prefix}/{url}");
        let mut request_builder = self.web_client.request(http_method, url);
        if let Some(params) = params {
            request_builder = request_builder.query(&params);
        }
        match body {
            Some(Body::Json(body)) => {
                request_builder = request_builder.json(&body);
            }
            Some(Body::UrlEncoded(body)) => {
                request_builder = request_builder.form(&body);
            }
            None => {}
        }
        match auth {
            AuthorizationType::None => {}
            AuthorizationType::Bearer(token) => {
                request_builder = request_builder.bearer_auth(token);
            }
            AuthorizationType::Basic { username, password } => {
                request_builder = request_builder.basic_auth(username, Some(password));
            }
        }
        let res = request_builder.send().await?;
        Ok(res.json().await?)
    }
}

impl Webex {
    /// Constructs a new Webex Teams context from a token
    /// Tokens can be obtained when creating a bot, see <https://developer.webex.com/my-apps> for
    /// more information and to create your own Webex bots.
    pub async fn new(token: &str) -> Self {
        Self::new_with_device_name(DEFAULT_DEVICE_NAME, token).await
    }

    /// Constructs a new Webex Teams context from a token and a chosen name
    /// The name is used to identify the device/client with Webex api
    pub async fn new_with_device_name(device_name: &str, token: &str) -> Self {
        let mut client: RestClient = RestClient {
            host_prefix: HashMap::new(),
            web_client: reqwest::Client::new(),
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
                device_name: Some(DEFAULT_DEVICE_NAME.to_string()),
                device_type: Some("DESKTOP".to_string()),
                localized_model: Some("rust".to_string()),
                model: Some(format!("rust-v{CRATE_VERSION}")),
                name: Some(device_name.to_owned()),
                system_name: Some(DEVICE_SYSTEM_NAME.to_string()),
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
            trace!("Attempting connection with device named {:?}", device.name);
            let Some(ws_url) = device.ws_url else {
                return Err("Device has no ws_url".into());
            };
            let url = url::Url::parse(ws_url.as_str())
                .map_err(|_| Error::from("Failed to parse ws_url"))?;
            debug!("Connecting to {:?}", url);
            match connect_async(url.as_str()).await {
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
                    Err(Error::Tungstenite(
                        e,
                        "Failed to connect to ws_url".to_string(),
                    ))
                }
            }
        }

        // get_devices automatically tries to set up devices if the get fails.
        // Keep only devices named DEVICE_NAME to avoid conflicts with other clients
        let mut devices: Vec<DeviceData> = self
            .get_devices()
            .await?
            .iter()
            .filter(|d| d.name == self.device.name)
            .inspect(|d| trace!("Kept device: {}", d))
            .cloned()
            .collect();

        // Sort devices in descending order by modification time, meaning latest created device
        // first.
        devices.sort_by(|a: &DeviceData, b: &DeviceData| {
            b.modification_time
                .unwrap_or_else(chrono::Utc::now)
                .cmp(&a.modification_time.unwrap_or_else(chrono::Utc::now))
        });

        for device in devices {
            if let Ok(event_stream) = connect_device(self, device).await {
                trace!("Successfully connected to device.");
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
            .map(|cache| cache.get(&self.id).cloned())
        {
            trace!("Found mercury URL in cache!");
            return result.map_err(|()| None);
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
        let api_url = "limited/catalog";
        let params = [("format", "hostmap"), ("orgId", org_id.as_str())];
        let catalogs = self
            .client
            .api_get::<CatalogReply>(
                api_url,
                Some(params),
                AuthorizationType::Bearer(&self.token),
            )
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
        let futures: Vec<_> = teams
            .into_iter()
            .map(|team| {
                let params = [("teamId", team.id)];
                self.client.api_get::<ListResult<Room>>(
                    Room::API_ENDPOINT,
                    Some(params),
                    AuthorizationType::Bearer(&self.token),
                )
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
    ///   `to_person_id` or `to_person_email`.
    ///
    /// # Errors
    /// Types of errors returned:
    /// * [`Error::Limited`] - returned on HTTP 423/429 with an optional Retry-After.
    /// * [`Error::Status`] | [`Error::StatusText`] - returned when the request results in a non-200 code.
    /// * [`Error::Json`] - returned when your input object cannot be serialized, or the return
    ///   value cannot be deserialised. (If this happens, this is a library bug and should be
    ///   reported.)
    /// * [`Error::UTF8`] - returned when the request returns non-UTF8 code.
    pub async fn send_message(&self, message: &MessageOut) -> Result<Message, Error> {
        self.client
            .api_post(
                "messages",
                message,
                None::<()>,
                AuthorizationType::Bearer(&self.token),
            )
            .await
    }

    /// Edit an existing message
    ///
    /// # Arguments
    /// * `params`: [`MessageEditParams`] - the message to edit, including the message ID and the room ID,
    ///   as well as the new message text.
    ///
    /// # Errors
    /// Types of errors returned:
    /// * [`Error::Limited`] - returned on HTTP 423/429 with an optional Retry-After.
    /// * [`Error::Status`] | [`Error::StatusText`] - returned when the request results in a non-200 code.
    /// * [`Error::Json`] - returned when your input object cannot be serialized, or the return
    ///   value cannot be deserialised. (If this happens, this is a library bug and should be reported).
    pub async fn edit_message(
        &self,
        message_id: &GlobalId,
        params: &MessageEditParams<'_>,
    ) -> Result<Message, Error> {
        let rest_method = format!("messages/{}", message_id.id());
        self.client
            .api_put(
                &rest_method,
                params,
                None::<()>,
                AuthorizationType::Bearer(&self.token),
            )
            .await
    }

    /// Get a resource from an ID
    /// # Errors
    /// * [`Error::Limited`] - returned on HTTP 423/429 with an optional Retry-After.
    /// * [`Error::Status`] | [`Error::StatusText`] - returned when the request results in a non-200 code.
    /// * [`Error::Json`] - returned when your input object cannot be serialized, or the return
    ///   value cannot be deserialised. (If this happens, this is a library bug and should be
    ///   reported.)
    /// * [`Error::UTF8`] - returned when the request returns non-UTF8 code.
    pub async fn get<T: Gettable + DeserializeOwned>(&self, id: &GlobalId) -> Result<T, Error> {
        let rest_method = format!("{}/{}", T::API_ENDPOINT, id.id());
        self.client
            .api_get::<T>(
                rest_method.as_str(),
                None::<()>,
                AuthorizationType::Bearer(&self.token),
            )
            .await
    }

    /// Delete a resource from an ID
    pub async fn delete<T: Gettable + DeserializeOwned>(&self, id: &GlobalId) -> Result<(), Error> {
        let rest_method = format!("{}/{}", T::API_ENDPOINT, id.id());
        self.client
            .api_delete(
                rest_method.as_str(),
                None::<()>,
                AuthorizationType::Bearer(&self.token),
            )
            .await
    }

    /// List resources of a type
    pub async fn list<T: Gettable + DeserializeOwned>(&self) -> Result<Vec<T>, Error> {
        self.client
            .api_get::<ListResult<T>>(
                T::API_ENDPOINT,
                None::<()>,
                AuthorizationType::Bearer(&self.token),
            )
            .await
            .map(|result| result.items)
    }

    /// List resources of a type, with parameters
    pub async fn list_with_params<T: Gettable + DeserializeOwned>(
        &self,
        list_params: T::ListParams<'_>,
    ) -> Result<Vec<T>, Error> {
        self.client
            .api_get::<ListResult<T>>(
                T::API_ENDPOINT,
                Some(list_params),
                AuthorizationType::Bearer(&self.token),
            )
            .await
            .map(|result| result.items)
    }

    async fn get_devices(&self) -> Result<Vec<DeviceData>, Error> {
        match self
            .client
            .api_get::<DevicesReply>(
                "devices",
                None::<()>,
                AuthorizationType::Bearer(&self.token),
            )
            .await
        {
            #[rustfmt::skip]
            Ok(DevicesReply { devices: Some(devices), .. }) => Ok(devices),
            Ok(DevicesReply { devices: None, .. }) => {
                debug!("Chaining one-time device setup from devices query");
                self.setup_devices().await.map(|device| vec![device])
            }
            Err(e) => match e {
                Error::Status(s) | Error::StatusText(s, _) => {
                    if s == StatusCode::NOT_FOUND {
                        debug!("No devices found, creating new one");
                        self.setup_devices().await.map(|device| vec![device])
                    } else {
                        Err(e)
                    }
                }
                Error::Limited(_, _) => Err(e),
                _ => Err(format!("Can't decode devices reply: {e}").into()),
            },
        }
    }

    async fn setup_devices(&self) -> Result<DeviceData, Error> {
        trace!("Setting up new device: {}", &self.device);
        self.client
            .api_post(
                "devices",
                &self.device,
                None::<()>,
                AuthorizationType::Bearer(&self.token),
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
            new_msg.room_id.clone_from(&msg.room_id);
        } else if let Some(_person_id) = &msg.person_id {
            new_msg.to_person_id.clone_from(&msg.person_id);
        } else {
            new_msg.to_person_email.clone_from(&msg.person_email);
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
        MessageOut {
            room_id: self.room_id.clone(),
            parent_id: self
                .parent_id
                .as_deref()
                .or(self.id.as_deref())
                .map(ToOwned::to_owned),
            ..Default::default()
        }
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
