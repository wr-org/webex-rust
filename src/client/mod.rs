//! Main Webex client implementation for interacting with the Webex Teams API.

use crate::adaptive_card::AdaptiveCard;
use crate::error::Error;
use crate::types::{
    Attachment, AttachmentAction, CatalogReply, DeviceData, DevicesReply, Gettable, GlobalId,
    GlobalIdType, ListResult, Membership, MembershipListParams, Message, MessageEditParams,
    MessageOut, Organization, Person, Room, RoomType, Team,
};
use futures::{future::try_join_all, try_join};
use log::{debug, error, trace, warn};
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{self, Hasher},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio_tungstenite::connect_async;

mod rest;
mod websocket;

pub use rest::{AuthorizationType, RestClient};
pub use websocket::{WStream, WebexEventStream};

// Re-export constants from parent
use super::{
    CRATE_VERSION, DEFAULT_DEVICE_NAME, DEFAULT_REGISTRATION_HOST_PREFIX, DEVICE_SYSTEM_NAME,
    REST_HOST_PREFIX, U2C_HOST_PREFIX,
};

/// Main client for interacting with the Webex Teams API.
///
/// This client handles authentication, REST API requests, and WebSocket event streams.
/// Create a new client using [`Webex::new`] with an API token.
///
/// # Example
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let webex = webex::Webex::new("YOUR_API_TOKEN").await;
/// let rooms = webex.list::<webex::Room>().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Webex {
    id: u64,
    client: RestClient,
    token: String,
    /// Webex Device Information used for device registration
    pub device: DeviceData,
    /// Cached user ID to avoid repeated /people/me calls
    user_id: Arc<Mutex<Option<String>>>,
}

/// Webex Event Stream handler
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
            user_id: Arc::new(Mutex::new(None)),
        };

        let devices_url = match webex.get_mercury_url().await {
            Ok(url) => {
                trace!("Fetched mercury url {url}");
                url
            }
            Err(e) => {
                debug!("Failed to fetch devices url, falling back to default");
                debug!("Error: {e:?}");
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
            debug!("Connecting to {url:?}");
            match connect_async(url.as_str()).await {
                Ok((mut ws_stream, _response)) => {
                    debug!("Connected to {url}");
                    WebexEventStream::auth(&mut ws_stream, &s.token).await?;
                    debug!("Authenticated");
                    let timeout = Duration::from_secs(20);
                    Ok(WebexEventStream::new(ws_stream, timeout))
                }
                Err(e) => {
                    warn!("Failed to connect to {url:?}: {e:?}");
                    Err(Error::Tungstenite(
                        Box::new(e),
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
            .inspect(|d| trace!("Kept device: {d}"))
            .cloned()
            .collect();

        // Sort devices in descending order by modification time, meaning latest created device
        // first. Use current time as fallback for devices without modification_time.
        let now = chrono::Utc::now();
        devices.sort_by(|a: &DeviceData, b: &DeviceData| {
            b.modification_time
                .unwrap_or(now)
                .cmp(&a.modification_time.unwrap_or(now))
        });

        for device in devices {
            if let Ok(event_stream) = connect_device(self, device).await {
                trace!("Successfully connected to device.");
                return Ok(event_stream);
            }
        }

        // Failed to connect to any existing devices, creating new one
        match self.setup_devices().await {
            Ok(device) => connect_device(self, device).await,
            Err(e) => match &e {
                Error::StatusText(status, _) if *status == StatusCode::FORBIDDEN => {
                    error!(
                        "Device creation failed with 403. Event stream requires OAuth scopes: \
                         spark:devices_write, spark:devices_read"
                    );
                    Err(e)
                }
                _ => {
                    error!("Failed to setup devices: {e}");
                    Err(e)
                }
            },
        }
    }

    async fn get_mercury_url(&self) -> Result<String, Option<Error>> {
        // Bit of a hacky workaround, error::Error does not implement clone
        // TODO: this can be fixed by returning a Result<String, &error::Error>
        static MERCURY_CACHE: std::sync::LazyLock<Mutex<HashMap<u64, Result<String, ()>>>> =
            std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));
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

    async fn get_mercury_url_uncached(&self) -> Result<String, Error> {
        // Steps:
        // 1. Get org id by GET /v1/organizations
        // 2. Get urls json from https://u2c.wbx2.com/u2c/api/v1/limited/catalog?orgId=[org id]
        // 3. mercury url is urls["serviceLinks"]["wdm"]
        //
        // 4. Add caching because this doesn't change, and it can be slow

        let orgs = match self.list::<Organization>().await {
            Ok(orgs) => orgs,
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("missing required scopes")
                    || error_msg.contains("missing required roles")
                {
                    debug!("Insufficient permissions to list organizations, falling back to default mercury URL");
                    return Err(
                        "Can't get mercury URL with insufficient organization permissions".into(),
                    );
                }
                return Err(e);
            }
        };
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
            all_rooms.extend(room.items.or(room.devices).unwrap_or_else(Vec::new));
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
            .map(|result| result.items.or(result.devices).unwrap_or_default())
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
            .map(|result| result.items.or(result.devices).unwrap_or_default())
    }

    /// Get the current user's ID, caching it for future calls
    ///
    /// # Errors
    /// * [`Error::Limited`] - returned on HTTP 423/429 with an optional Retry-After.
    /// * [`Error::Status`] | [`Error::StatusText`] - returned when the request results in a non-200 code.
    /// * [`Error::Json`] - returned when input/output cannot be serialized/deserialized.
    /// * [`Error::UTF8`] - returned when the request returns non-UTF8 code.
    async fn get_user_id(&self) -> Result<String, Error> {
        // Check if we already have the user ID cached
        if let Ok(guard) = self.user_id.lock() {
            if let Some(cached_id) = guard.as_ref() {
                return Ok(cached_id.clone());
            }
        }

        // Fetch the user ID from the API
        let me_global_id =
            GlobalId::new_with_cluster_unchecked(GlobalIdType::Person, "me".to_string(), None);
        let me = self.get::<Person>(&me_global_id).await?;

        // Cache it for future use
        if let Ok(mut guard) = self.user_id.lock() {
            *guard = Some(me.id.clone());
        }

        debug!("Cached user ID: {}", me.id);
        Ok(me.id)
    }

    /// Leave a room by deleting the current user's membership
    ///
    /// # Arguments
    /// * `room_id`: The ID of the room to leave
    ///
    /// # Errors
    /// * [`Error::UserError`] - returned when attempting to leave a 1:1 direct room (not supported by Webex API)
    /// * [`Error::Limited`] - returned on HTTP 423/429 with an optional Retry-After.
    /// * [`Error::Status`] | [`Error::StatusText`] - returned when the request results in a non-200 code.
    /// * [`Error::Json`] - returned when input/output cannot be serialized/deserialized.
    /// * [`Error::UTF8`] - returned when the request returns non-UTF8 code.
    ///
    /// # Note
    /// The Webex API does not support leaving or deleting 1:1 direct message rooms.
    /// This function will return an error for direct rooms. Only group rooms can be left.
    pub async fn leave_room(&self, room_id: &GlobalId) -> Result<(), Error> {
        debug!("Leaving room: {}", room_id.id());

        // First, get the room details to check if it's a direct room
        let room = self.get::<Room>(room_id).await?;

        // Check if this is a 1:1 direct room - these cannot be left via API
        if room.room_type == "direct" {
            return Err(Error::UserError(
                "Cannot leave a 1:1 direct message room. The Webex API does not support leaving or hiding direct rooms. Only group rooms can be left.".to_string()
            ));
        }

        // Get the current user ID (cached after first call)
        let my_user_id = self.get_user_id().await?;
        debug!("Current user ID: {my_user_id}");

        // Get memberships in this room - we can use personId filter to get just our membership
        let membership_params = MembershipListParams {
            room_id: Some(room_id.id()),
            person_id: Some(&my_user_id),
            ..Default::default()
        };

        debug!("Fetching membership for user {my_user_id} in room");
        let memberships = self
            .list_with_params::<Membership>(membership_params)
            .await?;

        debug!("Found {} matching memberships", memberships.len());

        let membership = memberships.into_iter().next().ok_or_else(|| {
            error!(
                "Could not find membership for user '{my_user_id}' in room. \
                 User may not be a member or membership data is stale."
            );
            Error::UserError("User is not a member of this room".to_string())
        })?;

        debug!("Found membership with ID: {}", membership.id);
        let membership_id = GlobalId::new(GlobalIdType::Membership, membership.id.clone())?;
        let rest_method = format!("memberships/{}", membership_id.id());

        self.client
            .api_delete(
                &rest_method,
                None::<()>,
                AuthorizationType::Bearer(&self.token),
            )
            .await?;
        debug!("Successfully left room: {}", room_id.id());

        Ok(())
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
            Err(e) => self.handle_get_devices_error(e).await,
        }
    }

    /// Handle errors when getting devices, with automatic fallback to device creation.
    ///
    /// This method implements the following logic:
    /// - 404 Not Found → Create a new device
    /// - 403 Forbidden → Log detailed OAuth scope error, attempt device creation
    /// - 429 Rate Limited → Pass through the error
    /// - Other errors → Log and return error
    async fn handle_get_devices_error(&self, e: Error) -> Result<Vec<DeviceData>, Error> {
        match e {
            Error::Status(status) | Error::StatusText(status, _) => {
                self.handle_device_status_error(status, e).await
            }
            Error::Limited(_, _) => Err(e),
            _ => {
                error!("Can't decode devices reply: {e}");
                Err(format!("Can't decode devices reply: {e}").into())
            }
        }
    }

    /// Handle HTTP status code errors when accessing device endpoints.
    async fn handle_device_status_error(
        &self,
        status: StatusCode,
        original_error: Error,
    ) -> Result<Vec<DeviceData>, Error> {
        match status {
            StatusCode::NOT_FOUND => {
                debug!("No devices found (404), will create new device");
                self.setup_devices().await.map(|device| vec![device])
            }
            StatusCode::FORBIDDEN => self.handle_device_forbidden_error(&original_error).await,
            _ => {
                error!("Unexpected HTTP status {status} when listing devices");
                Err(original_error)
            }
        }
    }

    /// Handle 403 Forbidden errors on device endpoints with detailed OAuth scope guidance.
    async fn handle_device_forbidden_error(
        &self,
        original_error: &Error,
    ) -> Result<Vec<DeviceData>, Error> {
        // Extract error details if available
        let details = match original_error {
            Error::StatusText(_, msg) => Some(msg.as_str()),
            _ => None,
        };

        // Log detailed error message with OAuth scope requirements
        let scope_info = if let Some(msg) = details {
            format!(
                "Device endpoint returned 403 Forbidden: {msg}. \
                 Token missing required OAuth scopes: spark:devices_write, spark:devices_read"
            )
        } else {
            "Device endpoint returned 403 Forbidden. \
             Token missing required OAuth scopes: spark:devices_write, spark:devices_read"
                .to_string()
        };
        error!("{scope_info}");

        // Attempt device creation anyway (sometimes list fails but create succeeds)
        match self.setup_devices().await {
            Ok(device) => {
                debug!("Surprisingly, device creation succeeded despite 403 on list");
                Ok(vec![device])
            }
            Err(setup_err) => {
                error!(
                    "Device creation failed: {setup_err}. Cannot proceed without device access."
                );
                Err(Error::Status(StatusCode::FORBIDDEN))
            }
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

#[cfg(test)]
#[allow(clippy::significant_drop_tightening)]
mod tests {
    use super::*;
    use mockito::ServerGuard;
    use serde_json::json;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Helper function to create a test Webex client with mocked `RestClient`
    fn create_test_webex_client(server: &ServerGuard) -> Webex {
        let mut host_prefix = HashMap::new();
        host_prefix.insert("people/me".to_string(), server.url());
        host_prefix.insert(
            "rooms/Y2lzY29zcGFyazovL3VzL1JPT00vMTIzNDU2NzgtMTIzNC0xMjM0LTEyMzQtMTIzNDU2Nzg5MDEy"
                .to_string(),
            server.url(),
        );
        host_prefix.insert("memberships".to_string(), server.url());
        host_prefix.insert("memberships/Y2lzY29zcGFyazovL3VzL01FTUJFUlNISVAvODc2NTQzMjEtNDMyMS00MzIxLTQzMjEtMjEwOTg3NjU0MzIx".to_string(), server.url());

        let rest_client = RestClient {
            host_prefix,
            web_client: reqwest::Client::new(),
        };

        let device = DeviceData {
            url: Some("test_url".to_string()),
            ws_url: Some("ws://test".to_string()),
            device_name: Some("test_device".to_string()),
            device_type: Some("DESKTOP".to_string()),
            localized_model: Some("rust-sdk-test".to_string()),
            modification_time: Some(chrono::Utc::now()),
            model: Some("rust-sdk-test".to_string()),
            name: Some(format!(
                "rust-sdk-test-{}",
                COUNTER.fetch_add(1, Ordering::SeqCst)
            )),
            system_name: Some("rust-sdk-test".to_string()),
            system_version: Some("0.1.0".to_string()),
        };

        Webex {
            id: 1,
            client: rest_client,
            token: "test_token".to_string(),
            device,
            user_id: Arc::new(Mutex::new(None)),
        }
    }

    #[tokio::test]
    async fn test_leave_room_success() {
        let mut server = mockito::Server::new_async().await;

        // Mock the GET /rooms/{id} API call to check room type
        let room_mock = server
            .mock("GET", "/rooms/Y2lzY29zcGFyazovL3VzL1JPT00vMTIzNDU2NzgtMTIzNC0xMjM0LTEyMzQtMTIzNDU2Nzg5MDEy")
            .match_header("authorization", "Bearer test_token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({
                "id": "Y2lzY29zcGFyazovL3VzL1JPT00vMTIzNDU2NzgtMTIzNC0xMjM0LTEyMzQtMTIzNDU2Nzg5MDEy",
                "title": "Test Room",
                "type": "group",
                "isLocked": false,
                "lastActivity": "2024-01-01T00:00:00.000Z",
                "creatorId": "test_person_id",
                "created": "2024-01-01T00:00:00.000Z"
            }).to_string())
            .create_async()
            .await;

        // Mock the people/me API call
        let people_mock = server
            .mock("GET", "/people/me")
            .match_header("authorization", "Bearer test_token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "id": "test_person_id",
                    "emails": ["test@example.com"],
                    "displayName": "Test User",
                    "orgId": "test_org_id",
                    "created": "2024-01-01T00:00:00.000Z",
                    "lastActivity": "2024-01-01T00:00:00.000Z",
                    "status": "active",
                    "type": "person"
                })
                .to_string(),
            )
            .create_async()
            .await;

        // Mock the membership list API call
        let membership_mock = server
            .mock("GET", "/memberships")
            .match_header("authorization", "Bearer test_token")
            .match_query(mockito::Matcher::UrlEncoded(
                "roomId".into(),
                "Y2lzY29zcGFyazovL3VzL1JPT00vMTIzNDU2NzgtMTIzNC0xMjM0LTEyMzQtMTIzNDU2Nzg5MDEy"
                    .into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "items": [{
                    "id": "87654321-4321-4321-4321-210987654321",
                    "roomId": "test_room_id",
                    "personId": "test_person_id",
                    "personEmail": "test@example.com",
                    "personDisplayName": "Test User",
                    "personOrgId": "test_org_id",
                    "isModerator": false,
                    "isMonitor": false,
                    "created": "2024-01-01T00:00:00.000Z"
                }]
            }"#,
            )
            .create_async()
            .await;

        // Mock the membership deletion API call
        let delete_mock = server
            .mock("DELETE", "/memberships/Y2lzY29zcGFyazovL3VzL01FTUJFUlNISVAvODc2NTQzMjEtNDMyMS00MzIxLTQzMjEtMjEwOTg3NjU0MzIx")
            .match_header("authorization", "Bearer test_token")
            .with_status(204)
            .with_body("")
            .create_async()
            .await;

        let webex_client = create_test_webex_client(&server);
        let room_id = GlobalId::new(
            GlobalIdType::Room,
            "12345678-1234-1234-1234-123456789012".to_string(),
        )
        .unwrap();

        let result = webex_client.leave_room(&room_id).await;

        if let Err(e) = &result {
            eprintln!("Error: {e}");
        }
        assert!(result.is_ok());
        room_mock.assert_async().await;
        people_mock.assert_async().await;
        membership_mock.assert_async().await;
        delete_mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_leave_room_user_not_member() {
        let mut server = mockito::Server::new_async().await;

        // Mock the GET /rooms/{id} API call to check room type
        let room_mock = server
            .mock("GET", "/rooms/Y2lzY29zcGFyazovL3VzL1JPT00vMTIzNDU2NzgtMTIzNC0xMjM0LTEyMzQtMTIzNDU2Nzg5MDEy")
            .match_header("authorization", "Bearer test_token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({
                "id": "Y2lzY29zcGFyazovL3VzL1JPT00vMTIzNDU2NzgtMTIzNC0xMjM0LTEyMzQtMTIzNDU2Nzg5MDEy",
                "title": "Test Room",
                "type": "group",
                "isLocked": false,
                "lastActivity": "2024-01-01T00:00:00.000Z",
                "creatorId": "test_person_id",
                "created": "2024-01-01T00:00:00.000Z"
            }).to_string())
            .create_async()
            .await;

        // Mock the people/me API call
        let people_mock = server
            .mock("GET", "/people/me")
            .match_header("authorization", "Bearer test_token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "id": "test_person_id",
                    "emails": ["test@example.com"],
                    "displayName": "Test User",
                    "orgId": "test_org_id",
                    "created": "2024-01-01T00:00:00.000Z",
                    "lastActivity": "2024-01-01T00:00:00.000Z",
                    "status": "active",
                    "type": "person"
                })
                .to_string(),
            )
            .create_async()
            .await;

        // Mock the membership list API call returning empty list
        let membership_mock = server
            .mock("GET", "/memberships")
            .match_query(mockito::Matcher::UrlEncoded(
                "roomId".into(),
                "Y2lzY29zcGFyazovL3VzL1JPT00vMTIzNDU2NzgtMTIzNC0xMjM0LTEyMzQtMTIzNDU2Nzg5MDEy"
                    .into(),
            ))
            .match_header("authorization", "Bearer test_token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "items": []
                })
                .to_string(),
            )
            .create_async()
            .await;

        let webex_client = create_test_webex_client(&server);
        let room_id = GlobalId::new(
            GlobalIdType::Room,
            "12345678-1234-1234-1234-123456789012".to_string(),
        )
        .unwrap();

        let result = webex_client.leave_room(&room_id).await;

        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.to_string(), "User is not a member of this room");
        }
        room_mock.assert_async().await;
        people_mock.assert_async().await;
        membership_mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_leave_room_api_error() {
        let mut server = mockito::Server::new_async().await;

        // Mock the GET /rooms/{id} API call to check room type
        let room_mock = server
            .mock("GET", "/rooms/Y2lzY29zcGFyazovL3VzL1JPT00vMTIzNDU2NzgtMTIzNC0xMjM0LTEyMzQtMTIzNDU2Nzg5MDEy")
            .match_header("authorization", "Bearer test_token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({
                "id": "Y2lzY29zcGFyazovL3VzL1JPT00vMTIzNDU2NzgtMTIzNC0xMjM0LTEyMzQtMTIzNDU2Nzg5MDEy",
                "title": "Test Room",
                "type": "group",
                "isLocked": false,
                "lastActivity": "2024-01-01T00:00:00.000Z",
                "creatorId": "test_person_id",
                "created": "2024-01-01T00:00:00.000Z"
            }).to_string())
            .create_async()
            .await;

        // Mock the people/me API call
        let people_mock = server
            .mock("GET", "/people/me")
            .match_header("authorization", "Bearer test_token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "id": "test_person_id",
                    "emails": ["test@example.com"],
                    "displayName": "Test User",
                    "orgId": "test_org_id",
                    "created": "2024-01-01T00:00:00.000Z",
                    "lastActivity": "2024-01-01T00:00:00.000Z",
                    "status": "active",
                    "type": "person"
                })
                .to_string(),
            )
            .create_async()
            .await;

        // Mock the membership list API call returning error
        let membership_mock = server
            .mock("GET", "/memberships")
            .match_query(mockito::Matcher::UrlEncoded(
                "roomId".into(),
                "Y2lzY29zcGFyazovL3VzL1JPT00vMTIzNDU2NzgtMTIzNC0xMjM0LTEyMzQtMTIzNDU2Nzg5MDEy"
                    .into(),
            ))
            .match_header("authorization", "Bearer test_token")
            .with_status(403)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "message": "Access denied",
                    "errors": []
                })
                .to_string(),
            )
            .create_async()
            .await;

        let webex_client = create_test_webex_client(&server);
        let room_id = GlobalId::new(
            GlobalIdType::Room,
            "12345678-1234-1234-1234-123456789012".to_string(),
        )
        .unwrap();

        let result = webex_client.leave_room(&room_id).await;

        assert!(result.is_err());
        room_mock.assert_async().await;
        people_mock.assert_async().await;
        membership_mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_leave_room_direct_room_error() {
        let mut server = mockito::Server::new_async().await;

        // Mock the GET /rooms/{id} API call - return a direct room
        let room_mock = server
            .mock("GET", "/rooms/Y2lzY29zcGFyazovL3VzL1JPT00vMTIzNDU2NzgtMTIzNC0xMjM0LTEyMzQtMTIzNDU2Nzg5MDEy")
            .match_header("authorization", "Bearer test_token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({
                "id": "Y2lzY29zcGFyazovL3VzL1JPT00vMTIzNDU2NzgtMTIzNC0xMjM0LTEyMzQtMTIzNDU2Nzg5MDEy",
                "title": "Direct Chat",
                "type": "direct",
                "isLocked": false,
                "lastActivity": "2024-01-01T00:00:00.000Z",
                "creatorId": "test_person_id",
                "created": "2024-01-01T00:00:00.000Z"
            }).to_string())
            .create_async()
            .await;

        let webex_client = create_test_webex_client(&server);
        let room_id = GlobalId::new(
            GlobalIdType::Room,
            "12345678-1234-1234-1234-123456789012".to_string(),
        )
        .unwrap();

        let result = webex_client.leave_room(&room_id).await;

        assert!(result.is_err());
        if let Err(error) = result {
            assert!(error
                .to_string()
                .contains("Cannot leave a 1:1 direct message room"));
        }
        room_mock.assert_async().await;
    }
}
