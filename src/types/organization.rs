//! Organization and team-related types for the Webex API.

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

/// Holds details about the organization an account belongs to.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Organization {
    /// Id of the org.
    pub id: String,
    /// Display name of the org
    pub display_name: Option<String>,
    /// Date and time the org was created
    pub created: String,
}

/// Holds details about a team that includes the account.
#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Team {
    /// Id of the team
    pub id: String,
    /// Name of the team
    pub name: Option<String>,
    /// Date and time the team was created
    pub created: String,
    /// Team description
    pub description: Option<String>,
}

/// Internal catalog reply wrapper.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CatalogReply {
    /// Service links catalog
    pub service_links: Catalog,
}

/// Service catalog with URLs for various Webex services.
#[allow(missing_docs)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Catalog {
    /// Atlas service URL
    pub atlas: String,
    /// Broadworks IDP proxy URL
    #[serde(rename = "broadworksIdpProxy")]
    pub broadworks_idp_proxy: String,
    /// Client logs service URL
    #[serde(rename = "clientLogs")]
    pub client_logs: String,
    /// Ecomm service URL
    pub ecomm: String,
    /// FMS service URL
    pub fms: String,
    /// ID broker service URL
    pub idbroker: String,
    /// ID broker guest service URL
    pub idbroker_guest: String,
    /// Identity service URL
    pub identity: String,
    /// Identity guest CS service URL
    pub identity_guest_cs: String,
    /// License service URL
    pub license: String,
    /// Meeting registry service URL
    #[serde(rename = "meetingRegistry")]
    pub meeting_registry: String,
    /// Metrics service URL
    pub metrics: String,
    /// OAuth helper service URL
    pub oauth_helper: String,
    /// Settings service URL
    pub settings_service: String,
    /// U2C service URL
    pub u2c: String,
    /// wdm is the url used for fetching devices.
    pub wdm: String,
    /// Web authentication service URL
    pub web_authentication: String,
    /// Webex App API service URL
    pub webex_appapi_service: String,
}

/// Destination for a `MessageOut`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Destination {
    /// Post a message in this room
    RoomId(String),
    /// Post a message to a person, using their user ID
    ToPersonId(String),
    /// Post a message to a person, using their email
    ToPersonEmail(String),
}
