//! Internal API traits and types.

use super::{
    attachment::AttachmentAction,
    membership::{Membership, MembershipListParams},
    message::{Message, MessageListParams},
    organization::{Organization, Team},
    person::Person,
    room::{Room, RoomListParams},
};

/// Trait for API types. Has to be public due to trait bounds limitations on webex API, but hidden
/// in a private module so users don't see it.
pub trait Gettable {
    /// Endpoint to query to perform an HTTP GET request with an id (to get an instance), or
    /// without an id (to list them).
    const API_ENDPOINT: &'static str;
    /// List parameters type for this gettable type.
    type ListParams<'a>: serde::Serialize;
}

/// Infallible type for API endpoints that don't support listing.
#[derive(serde::Serialize, Clone, Debug)]
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
    type ListParams<'a> = RoomListParams<'a>;
}

impl Gettable for Person {
    const API_ENDPOINT: &'static str = "people";
    type ListParams<'a> = Option<Infallible>;
}

impl Gettable for Team {
    const API_ENDPOINT: &'static str = "teams";
    type ListParams<'a> = Option<Infallible>;
}

impl Gettable for Membership {
    const API_ENDPOINT: &'static str = "memberships";
    type ListParams<'a> = MembershipListParams<'a>;
}

/// Result of listing API resources.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResult<T> {
    /// Items returned from the API.
    pub items: Option<Vec<T>>,
    /// Some API endpoints might return different field names (e.g., devices).
    pub devices: Option<Vec<T>>,
    /// Handle error cases - allow `dead_code` since these are for future API error handling
    #[allow(dead_code)]
    pub(crate) message: Option<String>,
    /// Errors returned from the API.
    #[allow(dead_code)]
    pub(crate) errors: Option<Vec<serde_json::Value>>,
}
