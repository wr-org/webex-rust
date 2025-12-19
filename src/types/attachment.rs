//! Attachment and attachment action types for the Webex API.

use crate::adaptive_card::AdaptiveCard;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::HashMap;

/// Attachment for a message (typically an Adaptive Card).
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    /// The content type of the attachment.
    #[serde(rename = "contentType")]
    pub content_type: String,
    /// Adaptive Card content.
    pub content: AdaptiveCard,
}

/// Attachment action details (when a user interacts with an Adaptive Card).
#[skip_serializing_none]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentAction {
    /// A unique identifier for the action.
    pub id: String,
    /// The type of action performed. Only 'submit' is currently supported.
    /// Required when posting an attachment.
    #[serde(rename = "type")]
    pub action_type: Option<String>,
    /// The parent message the attachment action was performed on.
    /// Required when posting an attachment.
    pub message_id: Option<String>,
    /// The action's inputs.
    /// Required when posting an attachment.
    pub inputs: Option<HashMap<String, serde_json::Value>>,
    /// The ID of the person who performed the action.
    pub person_id: Option<String>,
    /// The ID of the room the action was performed within.
    pub room_id: Option<String>,
    /// The date and time the action was created.
    pub created: Option<String>,
}
