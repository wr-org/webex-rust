use crate::adaptive_card::AdaptiveCard;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Debug)]
pub struct Room {
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub room_type: String,
    #[serde(rename = "isLocked")]
    pub is_locked: bool,
    #[serde(rename = "teamId")]
    pub team_id: Option<String>,
    #[serde(rename = "lastActivity")]
    pub last_activity: String,
    #[serde(rename = "creatorId")]
    pub creator_id: String,
    pub created: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RoomsReply {
    pub items: Vec<Room>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DirectMessage {
    pub id: String,
    #[serde(rename = "roomId")]
    pub room_id: String,
    #[serde(rename = "roomType")]
    pub room_type: String,
    pub text: String,
    pub markdown: Option<String>,
    pub files: Option<Vec<String>>,
    #[serde(rename = "personId")]
    pub person_id: String,
    #[serde(rename = "personEmail")]
    pub person_email: String,
    pub created: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DirectMessagesReply {
    pub items: Vec<DirectMessage>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct MessageOut {
    #[serde(rename = "roomId")]
    pub room_id: Option<String>,
    #[serde(rename = "toPersonId")]
    pub to_person_id: Option<String>,
    #[serde(rename = "toPersonEmail")]
    pub to_person_email: Option<String>,
    pub text: Option<String>,
    pub markdown: Option<String>,
    pub files: Option<Vec<String>>,
    #[serde(rename = "mentionedPeople")]
    pub mentioned_people: Option<Vec<String>>,
    #[serde(rename = "mentionedGroups")]
    pub mentioned_groups: Option<Vec<String>>,
    pub attachments: Option<Vec<Attachment>>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Message {
    pub id: Option<String>,
    #[serde(rename = "roomId")]
    pub room_id: Option<String>,
    #[serde(rename = "roomType")]
    pub room_type: Option<String>,
    #[serde(rename = "toPersonId")]
    pub to_person_id: Option<String>,
    #[serde(rename = "toPersonEmail")]
    pub to_person_email: Option<String>,
    pub text: Option<String>,
    pub markdown: Option<String>,
    pub files: Option<Vec<String>>,
    #[serde(rename = "personId")]
    pub person_id: Option<String>,
    #[serde(rename = "personEmail")]
    pub person_email: Option<String>,
    #[serde(rename = "mentionedPeople")]
    pub mentioned_people: Option<Vec<String>>,
    #[serde(rename = "mentionedGroups")]
    pub mentioned_groups: Option<Vec<String>>,
    pub attachments: Option<Vec<Attachment>>,
    pub created: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct MessagesReply {
    pub items: Vec<Message>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Error {
    pub description: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DevicesReply {
    pub devices: Option<Vec<DeviceData>>,
    pub message: Option<String>,
    pub errors: Option<Vec<Error>>,
    #[serde(rename = "trackingId")]
    pub tracking_id: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct DeviceData {
    pub url: Option<String>,
    #[serde(rename = "webSocketUrl")]
    pub ws_url: Option<String>,
    #[serde(skip_serializing)]
    pub services: Option<HashMap<String, String>>,
    #[serde(rename = "deviceName")]
    pub device_name: Option<String>,
    #[serde(rename = "deviceType")]
    pub device_type: Option<String>,
    #[serde(rename = "localizedModel")]
    pub localized_model: Option<String>,
    pub model: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "systemName")]
    pub system_name: String,
    #[serde(rename = "systemVersion")]
    pub system_version: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Authorization {
    pub id: String,
    #[serde(rename = "type")]
    pub _type: String,
    pub data: AuthToken,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct AuthToken {
    pub token: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Actor {
    pub id: String,
    #[serde(rename = "objectType")]
    pub object_type: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "orgId")]
    pub org_id: String,
    #[serde(rename = "emailAddress")]
    pub email_address: String,
    #[serde(rename = "entryUUID")]
    pub entry_uuid: String,
    #[serde(rename = "type")]
    pub actor_type: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct EventData {
    #[serde(rename = "eventType")]
    pub event_type: String,
    pub actor: Option<Actor>,
    #[serde(rename = "conversationId")]
    pub conversation_id: Option<String>,
    pub activity: Option<Activity>,
}

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
    pub target: Target,
    #[serde(rename = "clientTempId")]
    pub client_temp_id: Option<String>,
    #[serde(rename = "encryptionKeyUrl")]
    pub encryption_key_url: Option<String>,
    #[serde(rename = "vectorCounters")]
    pub vector_counters: Option<VectorCounters>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct VectorCounters {
    #[serde(rename = "sourceDC")]
    pub source_dc: String,
    pub counters: HashMap<String, i64>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Target {
    pub id: String,
    #[serde(rename = "objectType")]
    pub object_type: String,
    pub url: String,
    pub participants: MiscItems,
    pub activities: MiscItems,
    pub tags: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Object {
    #[serde(rename = "objectType")]
    pub object_type: String,
    pub content: Option<String>,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    pub mentions: Option<MiscItems>,
    pub inputs: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct MiscItems {
    pub items: Vec<MiscItem>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct MiscItem {
    pub id: String,
    #[serde(rename = "objectType")]
    pub object_type: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Event {
    pub id: String,
    pub data: EventData,
    pub timestamp: i64,
    #[serde(rename = "trackingId")]
    pub tracking_id: String,

    #[serde(rename = "alertType")]
    pub alert_type: String,
    pub headers: HashMap<String, String>,
    #[serde(rename = "sequenceNumber")]
    pub sequence_number: i64,
    #[serde(rename = "filterMessage")]
    pub filter_message: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Attachment {
    #[serde(rename = "contentType")]
    pub content_type: String,
    pub content: AdaptiveCard,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct AttachmentContent {
    pub body: Vec<AttachmentContentBody>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct AttachmentContentBody {
    #[serde(rename = "type")]
    pub body_type: String,
    pub text: String,
    pub size: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct AttachmentAction {
    pub id: String,
    #[serde(rename = "type")]
    pub action_type: String,
    #[serde(rename = "messageId")]
    pub message_id: String,
    pub inputs: HashMap<String, String>,
    #[serde(rename = "personId")]
    pub person_id: String,
    #[serde(rename = "roomId")]
    pub room_id: String,
    pub created: String,
}
