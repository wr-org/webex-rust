#![deny(missing_docs)]
//! Basic types for Webex Teams APIs

use crate::{adaptive_card::AdaptiveCard, error, error::ResultExt};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryFrom;
use uuid::Uuid;

pub(crate) use api::{Gettable, ListResult};

mod api {
    //! Private crate to hold all types that the user shouldn't have to interact with.
    use super::{AttachmentAction, Message, Organization, Person, Room, Team};

    /// Trait for API types. Has to be public due to trait bounds limitations on webex API, but hidden
    /// in a private crate so users don't see it.
    pub trait Gettable {
        /// Endpoint to query to perform an HTTP GET request with an id (to get an instance), or
        /// without an id (to list them).
        const API_ENDPOINT: &'static str;
        type ListParams<'a>: serde::Serialize;
    }

    #[derive(crate::types::Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct MessageListParams<'a> {
        room_id: &'a str,
        //#[serde(skip_serializing_if = "Option::is_none")] // Should be automatic
        parent_id: Option<&'a str>,
        #[serde(skip_serializing_if = "<[_]>::is_empty")]
        mentioned_people: &'a [&'a str],
        // TODO: before: string, beforeMessage: id, max: number
    }

    #[derive(crate::types::Serialize)]
    pub enum Infallible {}

    impl Gettable for Message {
        const API_ENDPOINT: &'static str = "messages";
        type ListParams<'a> = MessageListParams<'a>;
    }

    impl Gettable for Organization {
        const API_ENDPOINT: &'static str = "organizations";
        type ListParams<'a> = Option<Infallible>;
    }

    impl Gettable for AttachmentAction {
        const API_ENDPOINT: &'static str = "attachment/actions";
        type ListParams<'a> = Option<Infallible>;
    }

    impl Gettable for Room {
        const API_ENDPOINT: &'static str = "rooms";
        type ListParams<'a> = Option<Infallible>;
    }

    impl Gettable for Person {
        const API_ENDPOINT: &'static str = "people";
        type ListParams<'a> = Option<Infallible>;
    }

    impl Gettable for Team {
        const API_ENDPOINT: &'static str = "teams";
        type ListParams<'a> = Option<Infallible>;
    }

    #[derive(crate::types::Deserialize)]
    pub struct ListResult<T> {
        pub items: Vec<T>,
    }
}

/// Webex Teams room information
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Room {
    /// A unique identifier for the room.
    pub id: String,
    /// A user-friendly name for the room.
    pub title: String,
    /// The room type.
    ///
    /// direct - 1:1 room
    /// group - group room
    #[serde(rename = "type")]
    pub room_type: String,
    /// Whether the room is moderated (locked) or not.
    pub is_locked: bool,
    /// The ID for the team with which this room is associated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
    /// The date and time of the room's last activity.
    pub last_activity: String,
    /// The ID of the person who created this room.
    pub creator_id: String,
    /// The date and time the room was created.
    pub created: String,
}

/// Holds details about the organization an account belongs to.
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Organization {
    /// Id of the org.
    pub id: String,
    /// Display name of the org
    pub display_name: String,
    /// Date and time the org was created
    pub created: String,
}

#[derive(Deserialize, Serialize, Debug)]
/// Holds details about a team that includes the account.
pub struct Team {
    /// Id of the team
    pub id: String,
    /// Name of the team
    pub name: String,
    /// Date and time the team was created
    pub created: String,
    /// Team description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CatalogReply {
    pub service_links: Catalog,
}
#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct Catalog {
    pub atlas: String,
    #[serde(rename = "broadworksIdpProxy")]
    pub broadworks_idp_proxy: String,
    #[serde(rename = "clientLogs")]
    pub client_logs: String,
    pub ecomm: String,
    pub fms: String,
    pub idbroker: String,
    pub idbroker_guest: String,
    pub identity: String,
    pub identity_guest_cs: String,
    pub license: String,
    #[serde(rename = "meetingRegistry")]
    pub meeting_registry: String,
    pub metrics: String,
    pub oauth_helper: String,
    pub settings_service: String,
    pub u2c: String,
    /// wdm is the url used for fetching devices.
    pub wdm: String,
    pub web_authentication: String,
    pub webex_appapi_service: String,
}

/// Destination for a `MessageOut`
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum Destination {
    /// Post a message in this room
    RoomId(String),
    /// Post a message to a person, using their user ID
    ToPersonId(String),
    /// Post a message to a person, using their email
    ToPersonEmail(String),
}

/// Outgoing message
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct MessageOut {
    /// The parent message to reply to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    /// The room ID of the message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<String>,
    /// The person ID of the recipient when sending a private 1:1 message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_person_id: Option<String>,
    /// The email address of the recipient when sending a private 1:1 message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_person_email: Option<String>,
    // TODO - should we use globalIDs? We should check this field before the message is sent
    // rolls up room_id, to_person_id, and to_person_email all in one field :)
    //#[serde(flatten)]
    //pub deliver_to: Option<Destination>,
    /// The message, in plain text. If markdown is specified this parameter may be optionally used to provide alternate text for UI clients that do not support rich text. The maximum message length is 7439 bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// The message, in Markdown format. The maximum message length is 7439 bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
    /// The public URL to a binary file to be posted into the room. Only one file is allowed per message. Uploaded files are automatically converted into a format that all Webex Teams clients can render. For the supported media types and the behavior of uploads, see the [Message Attachments Guide](https://developer.webex.com/docs/api/basics#message-attachments).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,
    /// Content attachments to attach to the message. Only one card per message is supported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<Attachment>>,
}

/// Type of room
#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RoomType {
    #[default]
    /// 1:1 private chat
    Direct,
    /// Group room
    Group,
}

/// Webex Teams message information
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    /// The unique identifier for the message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The room ID of the message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<String>,
    /// The room type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_type: Option<RoomType>,
    /// The person ID of the recipient when sending a private 1:1 message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_person_id: Option<String>,
    /// The email address of the recipient when sending a private 1:1 message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_person_email: Option<String>,
    /// The message, in plain text. If markdown is specified this parameter may be optionally used to provide alternate text for UI clients that do not support rich text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// The message, in Markdown format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
    /// The text content of the message, in HTML format. This read-only property is used by the Webex Teams clients.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
    /// Public URLs for files attached to the message. For the supported media types and the behavior of file uploads, see Message Attachments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,
    /// The person ID of the message author.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person_id: Option<String>,
    /// The email address of the message author.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person_email: Option<String>,
    /// People IDs for anyone mentioned in the message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mentioned_people: Option<Vec<String>>,
    /// Group names for the groups mentioned in the message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mentioned_groups: Option<Vec<String>>,
    /// Message content attachments attached to the message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<Attachment>>,
    /// The date and time the message was created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    /// The date and time the message was updated, if it was edited.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    /// The ID of the "parent" message (the start of the reply chain)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct EmptyReply {}

/// API Error
#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug)]
pub struct Error {
    pub description: String,
}

#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct DevicesReply {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub devices: Option<Vec<DeviceData>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<Error>>,
    #[serde(rename = "trackingId", skip_serializing_if = "Option::is_none")]
    pub tracking_id: Option<String>,
}

#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeviceData {
    pub url: Option<String>,
    #[serde(rename = "webSocketUrl")]
    pub ws_url: Option<String>,
    #[serde(skip_serializing)]
    pub services: Option<HashMap<String, String>>,
    pub device_name: Option<String>,
    pub device_type: Option<String>,
    pub localized_model: Option<String>,
    pub capabilities: Option<DeviceCapabilities>,
    pub features: Option<DeviceFeatures>,
    pub creation_time: Option<chrono::DateTime<chrono::Utc>>,
    pub modification_time: Option<chrono::DateTime<chrono::Utc>>,
    pub device_settings_string: Option<String>,
    pub show_support_text: Option<bool>,
    pub reporting_site_url: Option<String>,
    pub reporting_site_desc: Option<String>,
    pub is_device_managed: Option<bool>,
    pub client_security_policy: Option<String>,
    pub intranet_inactivity_check_url: Option<String>,
    pub model: Option<String>,
    pub name: Option<String>,
    pub system_name: Option<String>,
    pub system_version: Option<String>,
    pub block_external_communications: Option<bool>,
    pub client_messaging_giphy: Option<String>,
    pub client_messaging_link_preview: Option<String>,
    pub ecm_enabled_for_all_users: Option<bool>,
    pub ecm_supported_storage_providers: Vec<String>,
    pub default_ecm_microsoft_cloud: Option<String>,
    pub ecm_microsoft_tenant: Option<String>,
    pub ecm_screen_capture_feature_allowed: Option<bool>,
    pub ecm_whiteboard_file_data_allowed: Option<bool>,
    pub calling_behavior: Option<String>,
    pub on_premise_pairing_enabled: Option<bool>,
    pub people_insights_enabled: Option<bool>,
    pub allow_self_signed_certificate: Option<bool>,
    pub webex_cross_launch: Option<bool>,
    pub settings: Option<DeviceSettings>,
    pub user_id: Option<String>,
    pub org_id: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs)]
pub struct DeviceFeatures {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub developer: Option<Vec<DeviceFeatureData>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entitlement: Option<Vec<DeviceFeatureData>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<Vec<DeviceFeatureData>>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs)]
pub struct DeviceFeatureData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub val: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_time: Option<i64>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs, clippy::struct_excessive_bools)]
pub struct DeviceCapabilities {
    pub group_call_supported: bool,
    pub local_notification_supported: bool,
    pub delete_notification_supported: bool,
    pub sdp_supported: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs, clippy::struct_excessive_bools)]
pub struct DeviceSettings {
    pub reporting_site_url: String,
    pub reporting_site_desc: String,
    pub show_support_text: bool,
    pub webex_cross_launch: bool,
    pub enable_intra_org_teams_call: bool,
    pub mobile_suppress_lock_screen_preview: bool,
    pub mobile_auto_lock_idle_timeout: i64,
    pub disable_meeting_scheduling: bool,
    pub ecm_enabled_for_all_users: bool,
    pub ecm_supported_storage_providers: Vec<String>,
    pub ecm_supported_folder_providers: Vec<String>,
    pub default_ecm_microsoft_cloud: String,
    pub ecm_microsoft_tenant: String,
    pub default_file_upload_location: String,
    pub restrict_accounts_to_email_domain: bool,
    pub ecm_screen_capture_feature_allowed: bool,
    pub ecm_whiteboard_file_data_allowed: bool,
    pub on_premise_pairing_enabled: bool,
    pub calling_behavior: String,
    pub client_messaging_giphy: String,
    pub client_messaging_link_preview: String,
    pub client_security_policy: String,
    pub intranet_inactivity_check_url: String,
    pub people_insights_enabled: bool,
    pub allow_self_signed_certificate: bool,
    pub block_external_communications: bool,
    pub reactions_enabled: bool,
    pub team_guest_member_restriction_enabled: bool,
    pub space_classifications_enabled: bool,
}

#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Authorization {
    pub id: String,
    #[serde(rename = "type")]
    pub auth_type: String,
    data: AuthToken,
}

impl Authorization {
    /// Create a new `Authorization` object from a token
    /// id is a random UUID v4
    #[must_use]
    pub fn new(token: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            auth_type: "authorization".to_string(),
            data: AuthToken {
                token: format!("Bearer {token}"),
            },
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub(crate) struct AuthToken {
    pub token: String,
}

#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Actor {
    pub id: String,
    pub object_type: String,
    pub display_name: Option<String>,
    pub org_id: Option<String>,
    pub email_address: Option<String>,
    #[serde(rename = "entryUUID")]
    pub entry_uuid: String,
    #[serde(rename = "type")]
    pub actor_type: Option<String>,
}

#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct EventData {
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<Actor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity: Option<Activity>,
}

#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub id: String,
    pub object_type: String,
    pub url: String,
    pub published: String,
    pub verb: String,
    pub actor: Actor,
    pub object: Object,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<Target>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_temp_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption_key_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector_counters: Option<VectorCounters>,
}

/// Get what activity an [`Activity`] represents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivityType {
    /// Message changed - see [`MessageActivity`] for details.
    Message(MessageActivity),
    /// The space the bot is in has changed - see [`SpaceActivity`] for details.
    Space(SpaceActivity),
    /// The user has submitted an [`AdaptiveCard`].
    AdaptiveCardSubmit,
    /// Meeting event.
    /// TODO: This needs to be broken down like `Message` and `Space`, if anyone cares.
    Locus,
    /// Call event.
    /// TODO: This may need to be broken down.
    /// May provide details about call insights/recording?
    Janus,
    /// Someone started typing.
    StartTyping,
    /// Not sure? perhaps when someone catches up in the conversation?
    Highlight,
    /// Unknown activity. Contains a representation of the string that failed to parse - unknown
    /// activities will contain `event.data.event_type`, otherwise if it's an Unknown
    /// `conversation.activity` type (belonging in Message or Space), the string will be
    /// `"conversation.activity.{event.data.activity.verb}"`, for example it would be
    /// `"conversation.activity.post"` for `Message(MessageActivity::Posted)`
    Unknown(String),
}
/// Specifics of what type of activity [`ActivityType::Message`] represents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageActivity {
    /// A message was posted
    Posted,
    /// A message was posted with attachments
    /// TODO: Should this be merged with [`Self::Posted`]? Could have a field to determine
    /// attachments/no attachments, or we can let the user figure that out from the message
    /// instance.
    Shared,
    /// A message was acknowledged
    Acknowledged,
    /// A message was deleted
    Deleted,
}
/// Specifics of what type of activity [`ActivityType::Space`] represents.
/// TODO: should we merge [`Self::Created`]/[`Self::Joined`]?
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpaceActivity {
    /// A new space was created with the bot
    Created,
    /// Bot was added to a space... or a reaction was added to a message?
    /// TODO: figure out a way to tell these events apart
    Joined,
    /// Bot left (was kicked out of) a space
    Left,
    /// Space was changed (i.e. name change, cover image changed, space picture changed).
    /// Also includes meeting changes (meeting name or schedule)
    Changed,
    /// New meeting scheduled
    MeetingScheduled,
    /// Space became moderated
    Locked,
    /// Space became unmoderated
    Unlocked,

    /// A new moderator was assigned
    ModeratorAssigned,
    /// A moderator was unassigned
    ModeratorUnassigned,
}
impl TryFrom<&str> for MessageActivity {
    type Error = ();
    fn try_from(s: &str) -> Result<Self, ()> {
        match s {
            "post" => Ok(Self::Posted),
            "share" => Ok(Self::Shared),
            "acknowledge" => Ok(Self::Acknowledged),
            "delete" => Ok(Self::Deleted),
            _ => Err(()),
        }
    }
}
impl TryFrom<&str> for SpaceActivity {
    type Error = ();
    fn try_from(s: &str) -> Result<Self, ()> {
        match s {
            "create" => Ok(Self::Created),
            "add" => Ok(Self::Joined),
            "leave" => Ok(Self::Left),
            "lock" => Ok(Self::Locked),
            "unlock" => Ok(Self::Unlocked),
            "update" | "assign" | "unassign" => Ok(Self::Changed),
            "schedule" => Ok(Self::MeetingScheduled),
            "assignModerator" => Ok(Self::ModeratorAssigned),
            "unassignModerator" => Ok(Self::ModeratorUnassigned),
            _ => Err(()),
        }
    }
}
impl MessageActivity {
    /// True if this is a new message ([`Self::Posted`] or [`Self::Shared`]).
    #[must_use]
    pub const fn is_created(&self) -> bool {
        matches!(*self, Self::Posted | Self::Shared)
    }
}

impl Event {
    /// Get the type of resource the event corresponds to.
    /// Also contains details about the event action for some event types.
    /// For more details, check [`ActivityType`].
    #[must_use]
    pub fn activity_type(&self) -> ActivityType {
        match self.data.event_type.as_str() {
            "conversation.activity" => {
                let activity_type = self
                    .data
                    .activity
                    .as_ref()
                    .expect("Conversation activity should have activity set")
                    .verb
                    .as_str();
                #[allow(clippy::option_if_let_else)]
                match activity_type {
                    // TODO: This probably has more options
                    // check self.data.activity.object.object_type == "submit"
                    "cardAction" => ActivityType::AdaptiveCardSubmit,
                    _ => {
                        // TODO: move these into their own `match` branches when we have
                        // match-if-let
                        // Tracking issue: https://github.com/rust-lang/rust/issues/51114
                        if let Ok(type_) = MessageActivity::try_from(activity_type) {
                            ActivityType::Message(type_)
                        } else if let Ok(type_) = SpaceActivity::try_from(activity_type) {
                            ActivityType::Space(type_)
                        } else {
                            log::error!(
                                "Unknown activity type `{}`, returning Unknown",
                                activity_type
                            );
                            ActivityType::Unknown(format!(
                                "conversation.activity.{}",
                                activity_type
                            ))
                        }
                    }
                }
            }
            "conversation.highlight" => ActivityType::Highlight,
            "status.start_typing" => ActivityType::StartTyping,
            "locus.difference" => ActivityType::Locus,
            "janus.user_sessions" => ActivityType::Janus,
            //"apheleia.subscription_update" ??
            e => {
                log::error!("Unknown data.event_type `{}`, returning Unknown", e);
                ActivityType::Unknown(e.to_string())
            }
        }
    }
    /// A function to extract a global ID from an activity.
    /// `event.data.activity.id` is a UUID, which can no longer be used for API requests, meaning any attempt
    /// at using this as an ID in a `Webex::get_*` will fail.
    /// Users should use this function to get a [`GlobalId`], which works with the updated API.
    pub fn get_global_id(&self) -> GlobalId {
        // Safety: ID should be fine since it's from the API (guaranteed to be UUID or b64 URI).
        //
        // NOTE: Currently uses None as default cluster
        // this means any UUID ID will default to cluster "us"
        // When we start supporting other clusters, if the API is still returning UUID URIs, we
        // need to investigate how to get the proper cluster. However, for now, the default is
        // always fine.
        // Note, we do not want to parse b64 URI into cluster, since cluster information is already
        // part of the URI and we don't need any additional information (the "cluster" argument is
        // ignored).
        let self_activity = self.data.activity.as_ref();
        GlobalId::new_with_cluster_unchecked(
            self.activity_type().into(),
            self_activity.map_or_else(|| self.id.clone(), |a| a.id.clone()),
            None,
        )
    }
}

/// This represents the type of an ID produced by the API, to prevent (for example) message IDs
/// being used for a room ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalIdType {
    /// This GlobalId represents the ID of a message
    Message,
    /// Corresponds to the ID of a person
    Person,
    /// Corresponds to the ID of a room
    Room,
    /// Retrieves a specific attachment
    AttachmentAction,
    /// This GlobalId represents the ID of something not currently recognised, any API requests
    /// with this GlobalId will produce an error.
    Unknown,
}
impl From<ActivityType> for GlobalIdType {
    fn from(a: ActivityType) -> Self {
        match a {
            ActivityType::Message(_) => Self::Message,
            ActivityType::AdaptiveCardSubmit => Self::AttachmentAction,
            ActivityType::Unknown(_) => Self::Unknown,
            a => {
                log::error!(
                    "Failed to convert {:?} to GlobalIdType, this may cause errors later",
                    a
                );
                Self::Unknown
            }
        }
    }
}
impl std::fmt::Display for GlobalIdType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "{}",
            match self {
                Self::Message => "MESSAGE",
                Self::Person => "PEOPLE",
                Self::Room => "ROOM",
                Self::AttachmentAction => "ATTACHMENT_ACTION",
                Self::Unknown => "<UNKNOWN>",
            }
        )
    }
}

/// This type is used to hold the ID of a message, room, person etc.
/// It is created from a certain resource type to make it impossible to use a person ID to fetch a
/// message, or vice versa.
#[derive(Debug, Clone)]
#[must_use]
pub struct GlobalId {
    id: String,
    type_: GlobalIdType,
}

impl GlobalId {
    /// Create a new ``GlobalId``, with an ID type as well as an API ID (which can be either old
    /// UUID-style, or new base64 URI style).
    pub fn new(type_: GlobalIdType, id: String) -> Result<Self, error::Error> {
        Self::new_with_cluster(type_, id, None)
    }
    /// Given an ID and a possible cluster, generate a new geo-ID.
    /// Will fail if given a ``GlobalIdType`` that doesn't correspond to a particular type (message, room,
    /// etc.)
    /// # Arguments
    /// * ``type_: GlobalIdType`` - the type of the ID being constructed
    /// * ``id: String`` - the ID, either old (UUID) or new (base64 geo-ID)
    /// * ``cluster: Option<&str>`` - cluster for geo-ID. Only used if the ID is an old-style UUID.
    /// Will default to `"us"` if not given and can't be determined from the ID - this should work
    /// for most requests.
    ///
    /// # Errors
    /// * ``ErrorKind::Msg`` if:
    ///   * the ID type is ``GlobalIdType::Unknown``.
    ///   * the ID is a base64 geo-ID that does not follow the format
    ///   ``ciscospark://[cluster]/[type]/[id]``.
    ///   * the ID is a base64 geo-ID and the type does not match the given type.
    ///   * the ID is a base64 geo-ID and the cluster does not match the given cluster.
    ///   * the ID is neither a UUID or a base64 geo-id.
    pub fn new_with_cluster(
        type_: GlobalIdType,
        id: String,
        cluster: Option<&str>,
    ) -> Result<Self, error::Error> {
        if type_ == GlobalIdType::Unknown {
            return Err("Cannot get globalId for unknown ID type".into());
        }
        if let Ok(decoded_id) = base64::engine::general_purpose::STANDARD.decode(&id) {
            let decoded_id = std::str::from_utf8(&decoded_id)
                .chain_err(|| "Failed to turn base64 id into UTF8 string")?;
            Self::check_id(decoded_id, cluster, &type_.to_string())?;
        } else if Uuid::parse_str(&id).is_err() {
            return Err("Expected ID to be base64 geo-id or uuid".into());
        }
        Ok(Self::new_with_cluster_unchecked(type_, id, cluster))
    }

    /// Given an ID and a possible cluster, generate a new geo-ID.
    /// Skips all checks. (If something wrong is passed, for example a [`GlobalIdType::Unknown`],
    /// this will silently produce a bad ID that will always return a 404 from the API.)
    pub fn new_with_cluster_unchecked(
        type_: GlobalIdType,
        id: String,
        cluster: Option<&str>,
    ) -> Self {
        let id = if Uuid::parse_str(&id).is_ok() {
            base64::engine::general_purpose::STANDARD.encode(format!(
                "ciscospark://{}/{}/{}",
                cluster.unwrap_or("us"),
                type_,
                id
            ))
        } else {
            id
        };
        Self { id, type_ }
    }
    fn check_id(id: &str, cluster: Option<&str>, type_: &str) -> Result<(), error::Error> {
        let decoded_parts: Vec<&str> = id.split('/').collect();
        if decoded_parts.len() != 5
            || decoded_parts[0] != "ciscospark:"
            || !decoded_parts[1].is_empty()
        {
            return Err(
                "Expected base64 ID to be in the form ciscospark://[cluster]/[type]/[id]".into(),
            );
        } else if let Some(expected_cluster) = cluster {
            if decoded_parts[2] != expected_cluster {
                // TODO - this won't happen when we fetch the cluster ourselves, since we get it from
                // the ID. Can we/should we skip this check somehow?

                return Err(format!(
                    "Expected base64 cluster to equal expected cluster {expected_cluster}"
                )
                .into());
            }
        } else if decoded_parts[3] != type_ {
            return Err(format!("Expected base64 type to equal {type_}").into());
        }
        Ok(())
    }
    /// Returns the base64 geo-ID as a ``&str`` for use in API requests.
    #[inline]
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Check if type is the same as expected type
    pub fn check_type(&self, expected_type: GlobalIdType) -> Result<(), error::Error> {
        if expected_type == self.type_ {
            Ok(())
        } else {
            Err(format!(
                "GlobalId type {} does not match expected type {expected_type}",
                self.type_
            )
            .into())
        }
    }
}

#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct VectorCounters {
    #[serde(rename = "sourceDC")]
    pub source_dc: String,
    pub counters: HashMap<String, i64>,
}

#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub id: String,
    pub object_type: String,
    pub url: String,
    pub participants: MiscItems,
    pub activities: MiscItems,
    pub tags: Vec<String>,
    pub global_id: String,
}

#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Object {
    pub object_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mentions: Option<MiscItems>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<String>,
}

#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct MiscItems {
    pub items: Vec<MiscItem>,
}

#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct MiscItem {
    pub id: String,
    #[serde(rename = "objectType")]
    pub object_type: String,
}

/// Alerting specified in received events.
/// TODO: may be missing some enum variants.
/// ALSO TODO: figure out what this does. Best guess, it refers to what alerts (e.g. a
/// notification) an event will generate.
/// There may be another variant for an event that may or may not make an alert (messages with
/// mentions?)
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "lowercase")]
pub enum AlertType {
    /// This event won't ever generate an alert (?)
    #[default]
    None,
    /// This event will always generate an alert (?)
    Full,
    /// okay, no idea...
    Visual,
}

/// Returned from [`WebexEventStream::next()`][`crate::WebexEventStream::next()`]. Contains information about the received event.
#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    /// Event ID, may be UUID or base64-encoded. Please do not use this directly, prefer to use
    /// [`Event::get_global_id()`].
    pub id: String,
    #[allow(missing_docs)]
    pub data: EventData,
    /// Timestamp in milliseconds since epoch.
    pub timestamp: i64,
    pub tracking_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alert_type: Option<AlertType>,
    pub headers: HashMap<String, String>,
    pub sequence_number: i64,
    pub filter_message: bool,
}

/// Message content attachments attached to the message.
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Attachment {
    /// The content type of the attachment.
    #[serde(rename = "contentType")]
    pub content_type: String,
    /// Adaptive Card content.
    pub content: AdaptiveCard,
}

/// Attachment action details
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentAction {
    /// A unique identifier for the action.
    pub id: String,
    /// The type of action performed.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub action_type: Option<String>,
    /// The parent message the attachment action was performed on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    /// The action's inputs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<HashMap<String, serde_json::Value>>,
    /// The ID of the person who performed the action.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person_id: Option<String>,
    /// The ID of the room the action was performed within.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<String>,
    /// The date and time the action was created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
}

/// Person information
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Person {
    /// A unique identifier for the person.
    pub id: String,
    /// The email addresses of the person.
    pub emails: Vec<String>,
    /// Phone numbers for the person.
    pub phone_numbers: Vec<PhoneNumber>,
    /// The full name of the person.
    pub display_name: String,
    /// The nickname of the person if configured. If no nickname is configured for the person, this field will not be present.
    pub nick_name: String,
    /// The first name of the person.
    pub first_name: String,
    /// The last name of the person.
    pub last_name: String,
    /// The URL to the person's avatar in PNG format.
    pub avatar: String,
    /// The ID of the organization to which this person belongs.
    pub org_id: String,
    /// The date and time the person was created.
    pub created: String,
    /// The date and time of the person's last activity within Webex Teams.
    pub last_activity: String,
    /// The current presence status of the person.
    ///
    /// active - active within the last 10 minutes
    /// call - the user is in a call
    /// DoNotDisturb - the user has manually set their status to "Do Not Disturb"
    /// inactive - last activity occurred more than 10 minutes ago
    /// meeting - the user is in a meeting
    /// OutOfOffice - the user or a Hybrid Calendar service has indicated that they are "Out of Office"
    /// pending - the user has never logged in; a status cannot be determined
    /// presenting - the user is sharing content
    /// unknown - the user’s status could not be determined
    pub status: String,
    /// The type of person account, such as person or bot.
    ///
    /// person- account belongs to a person
    /// bot - account is a bot user
    /// appuser - account is a guest user
    #[serde(rename = "type")]
    pub person_type: String,
}

/// Phone number information
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PhoneNumber {
    /// Phone number type
    #[serde(rename = "type")]
    pub number_type: String,
    /// Phone number
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn create_event(event_type: &str, activity_verb: &str) -> Event {
        Event {
            data: EventData {
                event_type: event_type.to_string(),
                activity: Some(Activity {
                    verb: activity_verb.to_string(),
                    ..Activity::default()
                }),
                ..EventData::default()
            },
            ..Event::default()
        }
    }

    #[test]
    fn event_parsing() {
        let test_events = [
            (
                "conversation.activity",
                "post",
                ActivityType::Message(MessageActivity::Posted),
            ),
            (
                "conversation.activity",
                "share",
                ActivityType::Message(MessageActivity::Shared),
            ),
            (
                "conversation.activity",
                "unknown",
                ActivityType::Unknown("conversation.activity.unknown".to_string()),
            ),
            ("unknown", "", ActivityType::Unknown("unknown".to_string())),
            ("conversation.highlight", "", ActivityType::Highlight),
        ];
        for test_e in test_events {
            let event = create_event(test_e.0, test_e.1);
            let result = test_e.2;
            assert_eq!(event.activity_type(), result);
        }
    }

    #[test]
    fn msg_is_created() {
        assert!(MessageActivity::Posted.is_created());
        assert!(MessageActivity::Shared.is_created());
        assert!(!MessageActivity::Deleted.is_created());
    }
}
