//! Main Webex client implementation for interacting with the Webex Teams API.

use crate::adaptive_card::AdaptiveCard;
use crate::error::Error;
use crate::types::{
    Attachment, AttachmentAction, CatalogReply, Continuation, DeviceData, DevicesReply, Gettable,
    GlobalId, GlobalIdType, ListResult, Membership, MembershipListParams, Message,
    MessageEditParams, MessageOut, Organization, Page, PaginationOptions, Person, Room, RoomType,
    Team,
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
            trace!("Saving mercury url to cache: {}=>{result:?}", self.id);
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

    /// Get all rooms, including rooms discovered through team enumeration.
    ///
    /// Enumerates all pages of rooms, teams, and each team's rooms using the
    /// default [`PaginationOptions`]. Existing ordering and duplicates are
    /// preserved. This may perform many requests; team requests run concurrently.
    pub async fn get_all_rooms(&self) -> Result<Vec<Room>, Error> {
        self.get_all_rooms_with_options(PaginationOptions::default())
            .await
    }

    /// Like [`Self::get_all_rooms`], with a page budget per enumeration
    /// (the initial rooms list, teams list, and each team's rooms list).
    /// This is not a global request budget across all teams.
    pub async fn get_all_rooms_with_options(
        &self,
        options: PaginationOptions,
    ) -> Result<Vec<Room>, Error> {
        let (mut all_rooms, teams) =
            try_join!(self.list_all(options), self.list_all::<Team>(options))?;
        let futures: Vec<_> = teams
            .into_iter()
            .map(|team| {
                let params = [("teamId", team.id)];
                self.client.api_list_all::<Room>(
                    Some(params),
                    AuthorizationType::Bearer(&self.token),
                    options,
                )
            })
            .collect();
        let teams_rooms = try_join_all(futures).await?;
        for rooms in teams_rooms {
            all_rooms.extend(rooms);
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

    /// List one page of resources, preserving the original bounded behavior.
    ///
    /// Discards continuation headers. Use [`Self::list_page`] for caller-controlled
    /// pagination, or [`Self::list_all`] for explicit complete enumeration.
    pub async fn list<T: Gettable + DeserializeOwned>(&self) -> Result<Vec<T>, Error> {
        let result: ListResult<T> = self
            .client
            .api_get(
                T::API_ENDPOINT,
                None::<()>,
                AuthorizationType::Bearer(&self.token),
            )
            .await?;
        Ok(result.items.or(result.devices).unwrap_or_default())
    }

    /// List one page with filters. `max` is Webex's page size, not a total limit.
    /// Continuation headers are discarded, just as in [`Self::list`].
    pub async fn list_with_params<T: Gettable + DeserializeOwned>(
        &self,
        list_params: T::ListParams<'_>,
    ) -> Result<Vec<T>, Error> {
        let result: ListResult<T> = self
            .client
            .api_get(
                T::API_ENDPOINT,
                Some(list_params),
                AuthorizationType::Bearer(&self.token),
            )
            .await?;
        Ok(result.items.or(result.devices).unwrap_or_default())
    }

    /// Fetch the first page without automatically requesting subsequent pages.
    ///
    /// An empty page can have a continuation. Only absence of a continuation
    /// signals completion. Malformed or unsafe links return [`Error::InvalidPagination`].
    ///
    /// ```no_run
    /// # async fn example(webex: &webex::Webex) -> Result<(), webex::error::Error> {
    /// let mut page = webex.list_page::<webex::Room>().await?;
    /// loop {
    ///     for room in page.items { println!("{:?}", room.title); }
    ///     // The application can stop here, keeping the continuation for later.
    ///     let Some(next) = page.continuation else { break };
    ///     page = webex.list_next_page(&next).await?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_page<T: Gettable + DeserializeOwned>(&self) -> Result<Page<T>, Error> {
        self.client
            .api_get_page(None::<()>, AuthorizationType::Bearer(&self.token))
            .await
    }

    /// Fetch the first page with filters and Webex's `max` page size.
    /// These parameters are sent only on this request; subsequent pages preserve
    /// the server's cursor and filters without appending the original parameters.
    pub async fn list_page_with_params<T: Gettable + DeserializeOwned>(
        &self,
        list_params: T::ListParams<'_>,
    ) -> Result<Page<T>, Error> {
        self.client
            .api_get_page(Some(list_params), AuthorizationType::Bearer(&self.token))
            .await
    }

    /// Fetch one continuation obtained from this client's page APIs.
    ///
    /// The resource type is inferred from the opaque continuation. Destinations
    /// and HTTP redirects are validated before sending credentials: the configured
    /// origin is trusted, and HTTPS `api.ciscospark.com` and `webexapis.com` are
    /// accepted as aliases when using either public API host. Other origins,
    /// userinfo, fragments, and HTTPS downgrades are rejected. No retries occur.
    /// Manual callers choose when to stop and must enforce their own budgets.
    pub async fn list_next_page<T: Gettable + DeserializeOwned>(
        &self,
        continuation: &Continuation<T>,
    ) -> Result<Page<T>, Error> {
        self.client
            .api_next_page(continuation, AuthorizationType::Bearer(&self.token))
            .await
    }

    /// Explicitly collect all pages in server order, including empty intermediate
    /// pages, within a caller-selected budget.
    ///
    /// Returns [`Error::PaginationLimit`] on budget exhaustion and
    /// [`Error::PaginationLoop`] on repeated continuations. Errors discard the
    /// collected result; use manual pages to retain progress. No retries occur.
    ///
    /// ```no_run
    /// # async fn example(webex: &webex::Webex) -> Result<(), webex::error::Error> {
    /// let rooms = webex.list_all::<webex::Room>(webex::PaginationOptions {
    ///     max_pages: 2_000,
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_all<T: Gettable + DeserializeOwned>(
        &self,
        options: PaginationOptions,
    ) -> Result<Vec<T>, Error> {
        self.client
            .api_list_all(None::<()>, AuthorizationType::Bearer(&self.token), options)
            .await
    }

    /// Collect all pages with filters and a page budget. `max` remains the server
    /// page size, so this may return more than `max` resources. Parameters are
    /// applied only to the first page, as in [`Self::list_page_with_params`].
    pub async fn list_all_with_params<T: Gettable + DeserializeOwned>(
        &self,
        list_params: T::ListParams<'_>,
        options: PaginationOptions,
    ) -> Result<Vec<T>, Error> {
        self.client
            .api_list_all(
                Some(list_params),
                AuthorizationType::Bearer(&self.token),
                options,
            )
            .await
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
        trace!("Setting up new device: {}", self.device);
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
    use mockito::Matcher;
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

    /// Build a minimal `Webex` whose `RestClient.host_prefix` routes the given
    /// endpoint paths at the mock server. Pagination follows absolute URLs
    /// returned in `Link` headers and therefore needs no prefix mapping for
    /// subsequent pages.
    fn paginated_webex(server: &ServerGuard, endpoints: &[&str]) -> Webex {
        let mut host_prefix = HashMap::new();
        for ep in endpoints {
            host_prefix.insert((*ep).to_string(), server.url());
        }
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
                "rust-sdk-test-pagination-{}",
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

    fn room_json(id: &str, title: &str) -> serde_json::Value {
        json!({
            "id": id,
            "title": title,
            "type": "group",
            "isLocked": false,
            "lastActivity": "2024-01-01T00:00:00.000Z",
            "creatorId": "test_person_id",
            "created": "2024-01-01T00:00:00.000Z",
        })
    }

    #[tokio::test]
    async fn existing_list_methods_request_only_one_page() {
        let mut server = mockito::Server::new_async().await;
        let first = server
            .mock("GET", "/rooms")
            .match_query(Matcher::Missing)
            .with_header("Link", "</rooms?cursor=next>; rel=next")
            .with_body(json!({"items": [room_json("r1", "Room")]}).to_string())
            .expect(1)
            .create_async()
            .await;
        let next = server
            .mock("GET", "/rooms?cursor=next")
            .expect(0)
            .create_async()
            .await;
        let messages = server
            .mock("GET", "/messages?roomId=history&max=1")
            .with_header("Link", "</messages?cursor=more>; rel=next")
            .with_body(r#"{"items":[{"id":"m1"}]}"#)
            .expect(1)
            .create_async()
            .await;
        let more = server
            .mock("GET", "/messages?cursor=more")
            .expect(0)
            .create_async()
            .await;
        let webex = paginated_webex(&server, &["rooms", "messages"]);
        assert_eq!(webex.list::<Room>().await.unwrap().len(), 1);
        let mut params = crate::MessageListParams::new("history");
        params.max = Some(1);
        assert_eq!(
            webex
                .list_with_params::<Message>(params)
                .await
                .unwrap()
                .len(),
            1
        );
        first.assert_async().await;
        next.assert_async().await;
        messages.assert_async().await;
        more.assert_async().await;
    }

    #[tokio::test]
    async fn manual_pages_preserve_parameters_credentials_and_server_cursor() {
        let mut server = mockito::Server::new_async().await;
        let first = server.mock("GET", "/rooms?teamId=T%26+1&max=1")
            .match_header("authorization", "Bearer test_token")
            .match_header("user-agent", format!("webex-rust/{CRATE_VERSION}").as_str())
            .with_header("Link", "</rooms?cursor=previous>; rel=prev")
            .with_header("Link", "</rooms?cursor=a%2Bb%2F%3D&teamId=server&max=2>; title=\"a,b; \\\"x\\\"\"; rel=\"next last\"")
            .with_body(json!({"items": [room_json("r1", "Room")]}).to_string())
            .expect(1).create_async().await;
        let second = server
            .mock("GET", "/rooms?cursor=a%2Bb%2F%3D&teamId=server&max=2")
            .match_header("authorization", "Bearer test_token")
            .match_header("user-agent", format!("webex-rust/{CRATE_VERSION}").as_str())
            .with_body(json!({"items": [room_json("r2", "Room")]}).to_string())
            .expect(1)
            .create_async()
            .await;
        let webex = paginated_webex(&server, &["rooms"]);
        let page = webex
            .list_page_with_params::<Room>(crate::RoomListParams {
                team_id: Some("T& 1"),
                max: Some(1),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(page.items[0].id, "r1");
        // Receiving a continuation does not fetch it.
        assert!(!second.matched_async().await);
        let page = webex
            .list_next_page(&page.continuation.unwrap())
            .await
            .unwrap();
        assert_eq!(page.items[0].id, "r2");
        assert!(page.continuation.is_none());
        first.assert_async().await;
        second.assert_async().await;
    }

    #[tokio::test]
    async fn all_pages_follow_empty_intermediate_pages() {
        let mut server = mockito::Server::new_async().await;
        let first = server
            .mock("GET", "/rooms?max=1")
            .with_header("Link", "</rooms?cursor=empty>; rel=next")
            .with_body(json!({"items": [room_json("r1", "Room")]}).to_string())
            .create_async()
            .await;
        let empty = server
            .mock("GET", "/rooms?cursor=empty")
            .with_header("Link", "</rooms?cursor=last>; rel=next")
            .with_body(r#"{"items":[]}"#)
            .create_async()
            .await;
        let last = server
            .mock("GET", "/rooms?cursor=last")
            .with_body(json!({"items": [room_json("r2", "Room")]}).to_string())
            .create_async()
            .await;
        let webex = paginated_webex(&server, &["rooms"]);
        let rooms = webex
            .list_all_with_params::<Room>(
                crate::RoomListParams {
                    max: Some(1),
                    ..Default::default()
                },
                PaginationOptions { max_pages: 3 },
            )
            .await
            .unwrap();
        assert_eq!(
            rooms.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            ["r1", "r2"]
        );
        first.assert_async().await;
        empty.assert_async().await;
        last.assert_async().await;
    }

    #[tokio::test]
    async fn distinct_pages_can_exceed_one_hundred() {
        let mut server = mockito::Server::new_async().await;
        let mut mocks = Vec::new();
        for page in 0..102 {
            let path = if page == 0 {
                "/messages".to_owned()
            } else {
                format!("/messages?cursor={page}")
            };
            let mut mock = server
                .mock("GET", path.as_str())
                .with_body(json!({"items":[{"id":page.to_string()}]}).to_string())
                .expect(1);
            if page < 101 {
                mock = mock.with_header(
                    "Link",
                    &format!("</messages?cursor={}>; rel=next", page + 1),
                );
            }
            mocks.push(mock.create_async().await);
        }
        let webex = paginated_webex(&server, &["messages"]);
        let items = webex
            .list_all::<Message>(PaginationOptions { max_pages: 102 })
            .await
            .unwrap();
        assert_eq!(items.len(), 102);
        assert_eq!(items.last().unwrap().id.as_deref(), Some("101"));
        for mock in mocks {
            mock.assert_async().await;
        }
    }

    #[tokio::test]
    async fn budgets_and_loops_have_distinct_errors() {
        let mut server = mockito::Server::new_async().await;
        let webex = paginated_webex(&server, &["rooms"]);
        let first = server
            .mock("GET", "/rooms")
            .match_query(Matcher::Missing)
            .with_header("Link", "</rooms?cursor=loop>; rel=next")
            .with_body(r#"{"items":[]}"#)
            .expect(2)
            .create_async()
            .await;
        let looping = server
            .mock("GET", "/rooms?cursor=loop")
            .with_header("Link", "</rooms?cursor=loop>; rel=next")
            .with_body(r#"{"items":[]}"#)
            .expect(1)
            .create_async()
            .await;
        assert!(matches!(
            webex
                .list_all::<Room>(PaginationOptions { max_pages: 0 })
                .await,
            Err(Error::PaginationLimit { max_pages: 0 })
        ));
        assert!(matches!(
            webex
                .list_all::<Room>(PaginationOptions { max_pages: 1 })
                .await,
            Err(Error::PaginationLimit { max_pages: 1 })
        ));
        assert!(matches!(
            webex
                .list_all::<Room>(PaginationOptions { max_pages: 10 })
                .await,
            Err(Error::PaginationLoop)
        ));
        first.assert_async().await;
        looping.assert_async().await;
    }

    #[tokio::test]
    async fn malformed_and_unsafe_links_fail_without_contacting_destination() {
        let mut server = mockito::Server::new_async().await;
        let mut unsafe_server = mockito::Server::new_async().await;
        let forbidden = unsafe_server
            .mock("GET", Matcher::Any)
            .expect(0)
            .create_async()
            .await;
        let webex = paginated_webex(&server, &["rooms"]);
        for link in [
            format!("<{}/rooms>; rel=next", unsafe_server.url()),
            "<https://webexapis.com.evil.example/rooms>; rel=next".into(),
            "not a real link".into(),
            "<http://[invalid>; rel=next".into(),
        ] {
            let first = server
                .mock("GET", "/rooms")
                .with_header("Link", &link)
                .with_body(r#"{"items":[]}"#)
                .create_async()
                .await;
            assert!(
                matches!(
                    webex.list_page::<Room>().await,
                    Err(Error::InvalidPagination(_))
                ),
                "{link}"
            );
            first.assert_async().await;
        }
        forbidden.assert_async().await;
    }

    #[tokio::test]
    async fn redirects_are_validated_before_forwarding_credentials() {
        let mut server = mockito::Server::new_async().await;
        let mut unsafe_server = mockito::Server::new_async().await;
        let forbidden = unsafe_server
            .mock("GET", Matcher::Any)
            .expect(0)
            .create_async()
            .await;
        let first = server
            .mock("GET", "/rooms")
            .match_header("authorization", "Bearer test_token")
            .with_status(302)
            .with_header("Location", "/rooms-redirected")
            .create_async()
            .await;
        let safe = server
            .mock("GET", "/rooms-redirected")
            .match_header("authorization", "Bearer test_token")
            .with_header("Link", "<?cursor=next>; rel=next")
            .with_body(r#"{"items":[]}"#)
            .create_async()
            .await;
        let unsafe_redirect = server
            .mock("GET", "/rooms-redirected?cursor=next")
            .match_header("authorization", "Bearer test_token")
            .with_status(307)
            .with_header("Location", &format!("{}/stolen", unsafe_server.url()))
            .create_async()
            .await;
        let webex = paginated_webex(&server, &["rooms"]);
        let page = webex.list_page::<Room>().await.unwrap();
        assert!(matches!(
            webex.list_next_page(&page.continuation.unwrap()).await,
            Err(Error::InvalidPagination(_))
        ));
        first.assert_async().await;
        safe.assert_async().await;
        unsafe_redirect.assert_async().await;
        forbidden.assert_async().await;
    }

    #[tokio::test]
    async fn get_all_rooms_paginates_team_rooms() {
        let mut server = mockito::Server::new_async().await;
        let team_next_url = format!("{}/rooms?teamId=T1&cursor=p2", server.url());

        // Initial rooms inventory also spans pages.
        let teamless = server
            .mock("GET", "/rooms")
            .match_query(mockito::Matcher::Missing)
            .match_header("authorization", "Bearer test_token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_header("Link", "</rooms?cursor=initial-next>; rel=next")
            .with_body(json!({ "items": [room_json("r0", "Teamless Room")] }).to_string())
            .create_async()
            .await;

        let initial_next = server
            .mock("GET", "/rooms?cursor=initial-next")
            .with_body(json!({"items": [room_json("r-extra", "Extra")]}).to_string())
            .create_async()
            .await;
        let teams_first = server
            .mock("GET", "/teams")
            .match_query(Matcher::Missing)
            .with_header("Link", "</teams?cursor=teams-next>; rel=next")
            .with_body(r#"{"items":[]}"#)
            .create_async()
            .await;
        // Teams list, second page
        let teams_mock = server
            .mock("GET", "/teams?cursor=teams-next")
            .match_header("authorization", "Bearer test_token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "items": [
                        { "id": "T1", "name": "Team 1", "created": "2024-01-01T00:00:00.000Z" }
                    ]
                })
                .to_string(),
            )
            .create_async()
            .await;

        // Team T1 rooms — page 1 with rel="next"
        let team_page1 = server
            .mock("GET", "/rooms")
            .match_query(mockito::Matcher::UrlEncoded("teamId".into(), "T1".into()))
            .match_header("authorization", "Bearer test_token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_header("Link", &format!("<{team_next_url}>; rel=\"next\""))
            .with_body(json!({ "items": [room_json("tr1", "Team Room 1")] }).to_string())
            .create_async()
            .await;

        // Team T1 rooms — page 2 (followed via absolute URL), no further Link
        let team_page2 = server
            .mock("GET", "/rooms")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("teamId".into(), "T1".into()),
                mockito::Matcher::UrlEncoded("cursor".into(), "p2".into()),
            ]))
            .match_header("authorization", "Bearer test_token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({ "items": [room_json("tr2", "Team Room 2")] }).to_string())
            .create_async()
            .await;

        let webex = paginated_webex(&server, &["rooms", "teams"]);
        let rooms = webex
            .get_all_rooms()
            .await
            .expect("get_all_rooms should succeed");

        let ids: Vec<String> = rooms.iter().map(|r| r.id.clone()).collect();
        assert!(
            ids.contains(&"r0".to_string()),
            "teamless room missing: {ids:?}"
        );
        assert!(
            ids.contains(&"tr1".to_string()),
            "team page 1 room missing: {ids:?}"
        );
        assert!(
            ids.contains(&"tr2".to_string()),
            "team page 2 room missing: {ids:?}"
        );
        assert!(ids.contains(&"r-extra".to_string()));
        assert_eq!(rooms.len(), 4, "unexpected rooms: {ids:?}");

        teamless.assert_async().await;
        initial_next.assert_async().await;
        teams_first.assert_async().await;
        teams_mock.assert_async().await;
        team_page1.assert_async().await;
        team_page2.assert_async().await;
    }
}
