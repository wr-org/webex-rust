//! Event and activity types for the Webex WebSocket API, including `GlobalId` utilities.

use crate::error;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::HashMap;
use std::convert::TryFrom;
use uuid::Uuid;

/// Actor information from WebSocket events.
#[allow(missing_docs)]
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
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
#[skip_serializing_none]
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventData {
    pub event_type: String,
    pub actor: Option<Actor>,
    pub conversation_id: Option<String>,
    pub activity: Option<Activity>,
}

#[allow(missing_docs)]
#[skip_serializing_none]
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityParent {
    pub actor_id: String,
    pub id: String,
    pub published: String,
    #[serde(rename = "type")]
    pub parent_type: String,
}

#[allow(missing_docs)]
#[skip_serializing_none]
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub actor: Actor,
    pub client_temp_id: Option<String>,
    pub encryption_key_url: Option<String>,
    pub id: String,
    pub object_type: String,
    pub object: Object,
    pub parent: Option<ActivityParent>,
    pub published: String,
    pub target: Option<Target>,
    pub url: Option<String>,
    pub vector_counters: Option<VectorCounters>,
    pub verb: String,
}

/// Get what activity an [`Activity`] represents.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActivityType {
    /// Message changed - see [`MessageActivity`] for details.
    Message(MessageActivity),
    /// The space the bot is in has changed - see [`SpaceActivity`] for details.
    Space(SpaceActivity),
    /// The user has submitted an [`AdaptiveCard`](crate::adaptive_card::AdaptiveCard).
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
#[derive(Clone, Debug, Eq, PartialEq)]
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpaceActivity {
    /// Space was changed (i.e. name change, cover image changed, space picture changed).
    /// Also includes meeting changes (meeting name or schedule)
    Changed,
    /// A new space was created with the bot
    Created,
    /// A space was favorited
    Favorite,
    /// Bot was added to a space... or a reaction was added to a message?
    /// TODO: figure out a way to tell these events apart
    Joined,
    /// Bot left (was kicked out of) a space
    Left,
    /// Space became moderated
    Locked,
    /// New meeting scheduled
    MeetingScheduled,
    /// A new moderator was assigned
    ModeratorAssigned,
    /// A moderator was unassigned
    ModeratorUnassigned,
    /// A space was unfavorited
    Unfavorite,
    /// Space became unmoderated
    Unlocked,
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
            "add" => Ok(Self::Joined),
            "assignModerator" => Ok(Self::ModeratorAssigned),
            "create" => Ok(Self::Created),
            "favorite" => Ok(Self::Favorite),
            "leave" => Ok(Self::Left),
            "lock" => Ok(Self::Locked),
            "schedule" => Ok(Self::MeetingScheduled),
            "unassignModerator" => Ok(Self::ModeratorUnassigned),
            "unfavorite" => Ok(Self::Unfavorite),
            "unlock" => Ok(Self::Unlocked),
            "update" | "assign" | "unassign" => Ok(Self::Changed),
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
    ///
    /// # Panics
    ///
    /// Will panic if conversation activity is not set
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
                                "Unknown activity type `{activity_type}`, returning Unknown"
                            );
                            ActivityType::Unknown(format!("conversation.activity.{activity_type}"))
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
                log::debug!("Unknown data.event_type `{e}`, returning Unknown");
                ActivityType::Unknown(e.to_string())
            }
        }
    }

    /// Extract a global ID from an activity.
    ///
    /// # Panics
    ///
    /// Will panic if the event is malformed and a global ID cannot be obtained.
    #[deprecated(since = "0.10.0", note = "please use `try_global_id` instead")]
    pub fn get_global_id(&self) -> GlobalId {
        self.try_global_id()
            .expect("Could not get global ID from event")
    }

    /// Extract a global ID from an activity.
    ///
    /// `event.data.activity.id` is a UUID, which can no longer be used for API requests, meaning any attempt
    /// at using this as an ID in a `Webex::get_*` will fail.
    /// Users should use this function to get a [`GlobalId`], which works with the updated API.
    pub fn try_global_id(&self) -> Result<GlobalId, crate::error::Error> {
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
        let activity = self
            .data
            .activity
            .as_ref()
            .ok_or(crate::error::Error::Api("Missing activity in event"))?;
        let id = match self.activity_type() {
            ActivityType::Space(SpaceActivity::Created) => self.room_id_of_space_created_event()?,
            ActivityType::Space(
                SpaceActivity::Changed | SpaceActivity::Joined | SpaceActivity::Left,
            )
            | ActivityType::Message(MessageActivity::Deleted) => Self::target_global_id(activity)?,
            _ => activity.id.clone(),
        };
        Ok(GlobalId::new_with_cluster_unchecked(
            self.activity_type().into(),
            id,
            None,
        ))
    }

    fn target_global_id(activity: &Activity) -> Result<String, error::Error> {
        activity
            .target
            .clone()
            .and_then(|t| t.global_id)
            .ok_or(crate::error::Error::Api("Missing target id in activity"))
    }

    /// Get the UUID of the room the Space created event corresponds to.
    /// This is a workaround for a bug in the API, where the UUID returned in the event is not correct.
    ///
    /// # Errors
    ///
    /// Returns an error if the event is not `Space::Created` or if activity is not set.
    fn room_id_of_space_created_event(&self) -> Result<String, crate::error::Error> {
        if self.activity_type() != ActivityType::Space(SpaceActivity::Created) {
            return Err(crate::error::Error::Api(
                "Expected space created event, got different activity type",
            ));
        }
        let activity_id = self
            .data
            .activity
            .clone()
            .ok_or(crate::error::Error::Api(
                "Missing activity in space created event",
            ))?
            .id;
        // If the id is not a UUID, assume it is already a correct global ID.
        // This could not be tested though as the API only returns UUID for now.
        if Uuid::parse_str(&activity_id).is_err() {
            return Ok(activity_id);
        }
        // API weirdness... the event contains an id that is close to the room id,
        // but it is not the same. It differs from the room id by one character,
        // always by a value of 2.
        let mut uuid = activity_id;
        if uuid.as_bytes()[7] == b'2' {
            uuid.replace_range(7..8, "0");
            Ok(uuid)
        } else {
            Err(crate::error::Error::Api(
                "Space created event uuid could not be not patched",
            ))
        }
    }
}

/// This represents the type of an ID produced by the API, to prevent (for example) message IDs
/// being used for a room ID.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GlobalIdType {
    /// This `GlobalId` represents the ID of a message
    Message,
    /// Corresponds to the ID of a person
    Person,
    /// Corresponds to the ID of a room
    Room,
    /// Corresponds to the ID of a team
    Team,
    /// Retrieves a specific attachment
    AttachmentAction,
    /// Corresponds to the ID of a membership
    Membership,
    /// This `GlobalId` represents the ID of something not currently recognised, any API requests
    /// with this `GlobalId` will produce an error.
    Unknown,
}
impl From<ActivityType> for GlobalIdType {
    fn from(a: ActivityType) -> Self {
        match a {
            ActivityType::AdaptiveCardSubmit => Self::AttachmentAction,
            ActivityType::Message(_) => Self::Message,
            ActivityType::Space(
                SpaceActivity::Changed
                | SpaceActivity::Created
                | SpaceActivity::Joined
                | SpaceActivity::Left,
            ) => Self::Room,
            ActivityType::Unknown(_) => Self::Unknown,
            a => {
                log::error!("Failed to convert {a:?} to GlobalIdType, this may cause errors later");
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
                Self::Team => "TEAM",
                Self::AttachmentAction => "ATTACHMENT_ACTION",
                Self::Membership => "MEMBERSHIP",
                Self::Unknown => "<UNKNOWN>",
            }
        )
    }
}

/// This type is used to hold the ID of a message, room, person etc.
/// It is created from a certain resource type to make it impossible to use a person ID to fetch a
/// message, or vice versa.
#[derive(Clone, Debug, Eq, PartialEq)]
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
    ///
    /// Will default to `"us"` if not given and can't be determined from the ID - this should work
    /// for most requests.
    ///
    /// # Errors
    /// * ``Error::Msg`` if:
    ///   * the ID type is ``GlobalIdType::Unknown``.
    ///   * the ID is a base64 geo-ID that does not follow the format ``ciscospark://[cluster]/[type]/[id]``.
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
        if let Ok(decoded_id) = base64::engine::general_purpose::STANDARD_NO_PAD.decode(&id) {
            let decoded_id = std::str::from_utf8(&decoded_id)?;
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
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VectorCounters {
    #[serde(rename = "sourceDC")]
    pub source_dc: String,
    pub counters: HashMap<String, i64>,
}

#[allow(missing_docs)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub id: String,
    pub object_type: String,
    pub url: String,
    pub participants: Option<MiscItems>,
    pub activities: Option<MiscItems>,
    pub tags: Vec<String>,
    pub global_id: Option<String>,
}

#[allow(missing_docs)]
#[skip_serializing_none]
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Object {
    pub object_type: String,
    pub content: Option<String>,
    pub display_name: Option<String>,
    pub mentions: Option<MiscItems>,
    pub inputs: Option<String>,
}

#[allow(missing_docs)]
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct MiscItems {
    #[serde(default)]
    pub items: Vec<MiscItem>,
}

#[allow(missing_docs)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MiscItem {
    pub id: String,
    #[serde(rename = "objectType")]
    pub object_type: String,
}

/// Alerting specified in received events.
///
/// TODO: may be missing some enum variants.
/// ALSO TODO: figure out what this does. Best guess, it refers to what alerts (e.g. a
/// notification) an event will generate.
/// There may be another variant for an event that may or may not make an alert (messages with
/// mentions?)
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
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
#[skip_serializing_none]
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
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
    pub alert_type: Option<AlertType>,
    pub headers: HashMap<String, String>,
    pub sequence_number: i64,
    pub filter_message: bool,
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

    #[test]
    fn global_id_without_padding() {
        // This is a real ID from the API, it does not have the final = padding.
        let id = "Y2lzY29zcGFyazovL3VzL1BFT1BMRS82YmIwODVmYS1mNmIyLTQyMTAtYjI2Ny1iZTBmZGViYjA3YzQ";
        let global_id = GlobalId::new(GlobalIdType::Person, id.to_string()).unwrap();
        assert_eq!(global_id.id(), id);
    }

    #[test]
    fn test_space_created_event_patched_room_id() {
        // patcheable UUID should return the correct room id
        let mut event = Event {
            id: "assumed_valid_base64".to_string(),
            data: EventData {
                event_type: "conversation.activity".to_string(),
                activity: Some(Activity {
                    verb: "create".to_string(),
                    id: "1ab849e2-9ab4-11ee-a70f-d9b57e49f8bf".to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(
            event.room_id_of_space_created_event().unwrap(),
            "1ab849e0-9ab4-11ee-a70f-d9b57e49f8bf"
        );
        // invalid UUID (assumed base64) should not be changed
        event.data.activity = Some(Activity {
            verb: "create".to_string(),
            id: "bogus".to_string(),
            ..Default::default()
        });
        assert_eq!(event.room_id_of_space_created_event().unwrap(), "bogus");
        // unpatcheable UUID should fail
        event.data.activity = Some(Activity {
            verb: "create".to_string(),
            id: "1ab849e9-9ab4-11ee-a70f-d9b57e49f8bf".to_string(),
            ..Default::default()
        });
        assert!(event.room_id_of_space_created_event().is_err());
    }

    #[test]
    fn test_global_id_from_uuid() {
        let uuid = "1ab849e0-9ab4-11ee-a70f-d9b57e49f8bf";
        let global_id = GlobalId::new(GlobalIdType::Room, uuid.to_string()).unwrap();

        assert_eq!(global_id.type_, GlobalIdType::Room);
        // The ID should be base64 encoded when created from a UUID
        assert!(!global_id.id().is_empty());
        assert_ne!(global_id.id(), uuid);
    }

    #[test]
    fn test_global_id_check_type_success() {
        let uuid = "1ab849e0-9ab4-11ee-a70f-d9b57e49f8bf";
        let global_id = GlobalId::new(GlobalIdType::Room, uuid.to_string()).unwrap();

        assert!(global_id.check_type(GlobalIdType::Room).is_ok());
    }

    #[test]
    fn test_global_id_check_type_failure() {
        let uuid = "1ab849e0-9ab4-11ee-a70f-d9b57e49f8bf";
        let global_id = GlobalId::new(GlobalIdType::Room, uuid.to_string()).unwrap();

        assert!(global_id.check_type(GlobalIdType::Person).is_err());
    }

    #[test]
    fn test_global_id_with_cluster() {
        let uuid = "1ab849e0-9ab4-11ee-a70f-d9b57e49f8bf";
        let global_id =
            GlobalId::new_with_cluster(GlobalIdType::Room, uuid.to_string(), Some("eu")).unwrap();

        // The cluster should be encoded in the base64 ID
        assert!(!global_id.id().is_empty());
        assert_ne!(global_id.id(), uuid);
    }

    #[test]
    fn test_global_id_unknown_type_error() {
        let uuid = "1ab849e0-9ab4-11ee-a70f-d9b57e49f8bf";
        let result = GlobalId::new(GlobalIdType::Unknown, uuid.to_string());

        assert!(result.is_err());
    }

    #[test]
    fn test_global_id_already_encoded() {
        // If given an already encoded GlobalId, it should pass through
        let encoded =
            "Y2lzY29zcGFyazovL3VzL1JPT00vMWFiODQ5ZTAtOWFiNC0xMWVlLWE3MGYtZDliNTdlNDlmOGJm";
        let global_id = GlobalId::new(GlobalIdType::Room, encoded.to_string()).unwrap();

        assert_eq!(global_id.id, encoded);
    }

    #[test]
    fn test_message_activity_is_created() {
        assert!(MessageActivity::Posted.is_created());
        assert!(!MessageActivity::Deleted.is_created());
    }
}
