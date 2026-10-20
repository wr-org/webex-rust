//! Device and authorization types for the Webex API.

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::fmt;
use uuid::Uuid;

/// Device error information.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeviceError {
    /// Error description
    pub description: String,
}

/// Internal devices reply wrapper.
#[allow(missing_docs)]
#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct DevicesReply {
    pub devices: Option<Vec<DeviceData>>,
    pub message: Option<String>,
    pub errors: Option<Vec<DeviceError>>,
    #[serde(rename = "trackingId")]
    pub tracking_id: Option<String>,
}

/// Webex device information.
#[allow(missing_docs)]
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceData {
    pub url: Option<String>,
    #[serde(rename = "webSocketUrl")]
    pub ws_url: Option<String>,
    pub device_name: Option<String>,
    pub device_type: Option<String>,
    pub localized_model: Option<String>,
    pub modification_time: Option<chrono::DateTime<chrono::Utc>>,
    pub model: Option<String>,
    pub name: Option<String>,
    pub system_name: Option<String>,
    pub system_version: Option<String>,
}

impl fmt::Display for DeviceData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "name: {:?}, device_name: {:?}, device_type: {:?}, model: {:?}, system_name: {:?}, system_version: {:?}, url: {:?}",
        self.name, self.device_name, self.device_type, self.model, self.system_name, self.system_version, self.url)
    }
}

/// Authorization token for WebSocket connection.
#[allow(missing_docs)]
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
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

/// Internal auth token wrapper.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct AuthToken {
    pub token: String,
}
