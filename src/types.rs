#![deny(missing_docs)]
//! Basic types for Webex Teams APIs

use crate::adaptive_card::AdaptiveCard;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Webex Teams room information
#[derive(Deserialize, Serialize, Debug)]
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
    #[serde(rename = "isLocked")]
    pub is_locked: bool,
    /// The ID for the team with which this room is associated.
    #[serde(rename = "teamId", skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
    /// The date and time of the room's last activity.
    #[serde(rename = "lastActivity")]
    pub last_activity: String,
    /// The ID of the person who created this room.
    #[serde(rename = "creatorId")]
    pub creator_id: String,
    /// The date and time the room was created.
    pub created: String,
}

/// API reply holding the room vector
#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug)]
pub struct RoomsReply {
    pub items: Vec<Room>,
}

/// Outgoing message
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct MessageOut {
    /// The parent message to reply to.
    #[serde(rename = "parentId", skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    /// The room ID of the message.
    #[serde(rename = "roomId", skip_serializing_if = "Option::is_none")]
    pub room_id: Option<String>,
    /// The person ID of the recipient when sending a private 1:1 message.
    #[serde(rename = "toPersonId", skip_serializing_if = "Option::is_none")]
    pub to_person_id: Option<String>,
    /// The email address of the recipient when sending a private 1:1 message.
    #[serde(rename = "toPersonEmail", skip_serializing_if = "Option::is_none")]
    pub to_person_email: Option<String>,
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

/// Webex Teams message information
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Message {
    /// The unique identifier for the message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The room ID of the message.
    #[serde(rename = "roomId", skip_serializing_if = "Option::is_none")]
    pub room_id: Option<String>,
    /// The room type.
    ///
    /// direct - 1:1 room
    /// group - group room
    #[serde(rename = "roomType", skip_serializing_if = "Option::is_none")]
    pub room_type: Option<String>,
    /// The person ID of the recipient when sending a private 1:1 message.
    #[serde(rename = "toPersonId", skip_serializing_if = "Option::is_none")]
    pub to_person_id: Option<String>,
    /// The email address of the recipient when sending a private 1:1 message.
    #[serde(rename = "toPersonEmail", skip_serializing_if = "Option::is_none")]
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
    #[serde(rename = "personId", skip_serializing_if = "Option::is_none")]
    pub person_id: Option<String>,
    /// The email address of the message author.
    #[serde(rename = "personEmail", skip_serializing_if = "Option::is_none")]
    pub person_email: Option<String>,
    /// People IDs for anyone mentioned in the message.
    #[serde(rename = "mentionedPeople", skip_serializing_if = "Option::is_none")]
    pub mentioned_people: Option<Vec<String>>,
    /// Group names for the groups mentioned in the message.
    #[serde(rename = "mentionedGroups", skip_serializing_if = "Option::is_none")]
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
}

/// API Message reply
#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug)]
pub struct MessagesReply {
    pub items: Vec<Message>,
}

/// API Empty reply
#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug)]
pub struct EmptyReply {}

/// API Error
#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug)]
pub struct Error {
    pub description: String,
}

#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug)]
pub struct DevicesReply {
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
#[allow(missing_docs)]
pub struct DeviceCapabilities {
    pub group_call_supported: bool,
    pub local_notification_supported: bool,
    pub delete_notification_supported: bool,
    pub sdp_supported: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs)]
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
    pub _type: String,
    pub data: AuthToken,
}

#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct AuthToken {
    pub token: String,
}

#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Actor {
    pub id: String,
    #[serde(rename = "objectType")]
    pub object_type: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "orgId")]
    pub org_id: Option<String>,
    #[serde(rename = "emailAddress")]
    pub email_address: Option<String>,
    #[serde(rename = "entryUUID")]
    pub entry_uuid: String,
    #[serde(rename = "type")]
    pub actor_type: Option<String>,
}

#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct EventData {
    #[serde(rename = "eventType")]
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<Actor>,
    #[serde(rename = "conversationId", skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity: Option<Activity>,
}

#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Activity {
    pub id: String,
    #[serde(rename = "objectType")]
    pub object_type: String,
    pub url: String,
    pub published: String,
    pub verb: String,
    pub actor: Actor,
    pub object: Object,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<Target>,
    #[serde(rename = "clientTempId", skip_serializing_if = "Option::is_none")]
    pub client_temp_id: Option<String>,
    #[serde(rename = "encryptionKeyUrl", skip_serializing_if = "Option::is_none")]
    pub encryption_key_url: Option<String>,
    #[serde(rename = "vectorCounters", skip_serializing_if = "Option::is_none")]
    pub vector_counters: Option<VectorCounters>,
}

#[allow(missing_docs)]
impl Activity {
    #[must_use]
    pub fn get_message_id(&self) -> MessageId {
        if let Some(target) = &self.target {
            MessageId {
                id: target.global_id.clone(),
            }
        } else {
            MessageId::new(self.id.clone())
        }
    }
}

#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct MessageId {
    id: String,
}

#[allow(missing_docs)]
impl From<String> for MessageId {
    fn from(s: String) -> MessageId {
        MessageId::new(s)
    }
}

#[allow(missing_docs)]
impl MessageId {
    #[must_use]
    pub fn new(id: String) -> Self {
        Self::new_with_cluster(id, None)
    }
    #[must_use]
    pub fn new_with_cluster(id: String, cluster: Option<String>) -> Self {
        let cluster = cluster.as_deref().unwrap_or("us");

        let id = match Uuid::parse_str(&id) {
            Ok(_) => base64::encode(format!("ciscospark://{}/MESSAGE/{}", cluster, id)),
            Err(_) => id,
        };
        Self { id }
    }
    #[must_use]
    pub fn id(&self) -> String {
        self.id.clone()
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
pub struct Target {
    pub id: String,
    #[serde(rename = "objectType")]
    pub object_type: String,
    pub url: String,
    pub participants: MiscItems,
    pub activities: MiscItems,
    pub tags: Vec<String>,
    #[serde(rename = "globalId")]
    pub global_id: String,
}

#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Object {
    #[serde(rename = "objectType")]
    pub object_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
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

#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Event {
    pub id: String,
    pub data: EventData,
    pub timestamp: i64,
    #[serde(rename = "trackingId")]
    pub tracking_id: String,
    #[serde(rename = "alertType", skip_serializing_if = "Option::is_none")]
    pub alert_type: Option<String>,
    pub headers: HashMap<String, String>,
    #[serde(rename = "sequenceNumber")]
    pub sequence_number: i64,
    #[serde(rename = "filterMessage")]
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
pub struct AttachmentAction {
    /// A unique identifier for the action.
    pub id: String,
    /// The type of action performed.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub action_type: Option<String>,
    /// The parent message the attachment action was performed on.
    #[serde(rename = "messageId", skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    /// The action's inputs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<HashMap<String, String>>,
    /// The ID of the person who performed the action.
    #[serde(rename = "personId", skip_serializing_if = "Option::is_none")]
    pub person_id: Option<String>,
    /// The ID of the room the action was performed within.
    #[serde(rename = "roomId", skip_serializing_if = "Option::is_none")]
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
#[serde(rename_all = "camelCase", default)]
pub struct PhoneNumber {
    /// Phone number type
    #[serde(rename = "type")]
    pub number_type: String,
    /// Phone number
    pub value: String,
}
