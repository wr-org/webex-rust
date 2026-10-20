//! Room-related types for the Webex API.

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

/// Webex Teams room information.
#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Room {
    /// A unique identifier for the room.
    pub id: String,
    /// A user-friendly name for the room.
    pub title: Option<String>,
    /// The room type.
    ///
    /// direct - 1:1 room
    /// group - group room
    #[serde(rename = "type")]
    pub room_type: String,
    /// Whether the room is moderated (locked) or not.
    pub is_locked: bool,
    /// The ID for the team with which this room is associated.
    pub team_id: Option<String>,
    /// The date and time of the room's last activity.
    pub last_activity: String,
    /// The ID of the person who created this room.
    pub creator_id: String,
    /// The date and time the room was created.
    pub created: String,
}

/// Sorting order for `RoomListParams`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SortRoomsBy {
    /// room id
    Id,
    /// last activity timestamp
    LastActivity,
    /// created timestamp
    Created,
}

/// Parameters for listing rooms.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomListParams<'a> {
    /// List rooms in a team, by ID.
    pub team_id: Option<&'a str>,
    /// List rooms by type. Cannot be set in combination with orgPublicSpaces.
    #[serde(rename = "type")]
    pub room_type: Option<RoomType>,
    /// Shows the org's public spaces joined and unjoined. When set the result list is sorted by the madePublic timestamp.
    pub org_public_spaces: Option<bool>,
    /// Filters rooms, that were made public after this time. See madePublic timestamp
    pub from: Option<&'a str>,
    /// Filters rooms, that were made public before this time. See madePublic timestamp
    pub to: Option<&'a str>,
    /// Sort results. Cannot be set in combination with orgPublicSpaces.
    pub sort_by: Option<SortRoomsBy>,
    /// Limit the maximum number of rooms in the response.
    /// Default: 100
    pub max: Option<u32>,
}

/// The type of room (direct message or group).
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RoomType {
    #[default]
    /// 1:1 private chat
    Direct,
    /// Group room
    Group,
}
