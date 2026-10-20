//! Membership-related types for the Webex API.

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

/// Webex Teams membership information.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Membership {
    /// A unique identifier for the membership.
    pub id: String,
    /// The room ID associated with this membership.
    #[serde(default, rename = "roomId")]
    pub room_id: String,
    /// The person ID associated with this membership.
    #[serde(default, rename = "personId")]
    pub person_id: String,
    /// The email address of the person.
    #[serde(rename = "personEmail")]
    pub person_email: Option<String>,
    /// The display name of the person.
    #[serde(rename = "personDisplayName")]
    pub person_display_name: Option<String>,
    /// The organization ID of the person.
    #[serde(rename = "personOrgId")]
    pub person_org_id: Option<String>,
    /// Whether or not the participant is a moderator of the room.
    #[serde(rename = "isModerator")]
    pub is_moderator: bool,
    /// Whether or not the participant is a monitor of the room.
    #[serde(rename = "isMonitor")]
    pub is_monitor: bool,
    /// The date and time when the membership was created.
    pub created: String,
}

/// Parameters for listing memberships.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MembershipListParams<'a> {
    /// List memberships for a room, by ID.
    pub room_id: Option<&'a str>,
    /// List memberships for a person, by ID.
    pub person_id: Option<&'a str>,
    /// List memberships for a person, by email address.
    pub person_email: Option<&'a str>,
    /// Limit the maximum number of memberships in the response.
    /// Default: 100
    pub max: Option<u32>,
}
