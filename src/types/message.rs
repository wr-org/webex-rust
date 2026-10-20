//! Message-related types for the Webex API.

use super::attachment::Attachment;
use super::room::RoomType;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

/// Outgoing message to be sent to Webex.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageOut {
    /// The parent message to reply to.
    pub parent_id: Option<String>,
    /// The room ID of the message.
    pub room_id: Option<String>,
    /// The person ID of the recipient when sending a private 1:1 message.
    pub to_person_id: Option<String>,
    /// The email address of the recipient when sending a private 1:1 message.
    pub to_person_email: Option<String>,
    // TODO - should we use globalIDs? We should check this field before the message is sent
    // rolls up room_id, to_person_id, and to_person_email all in one field :)
    //#[serde(flatten)]
    //pub deliver_to: Option<Destination>,
    /// The message, in plain text. If markdown is specified this parameter may be optionally used to provide alternate text for UI clients that do not support rich text. The maximum message length is 7439 bytes.
    pub text: Option<String>,
    /// The message, in Markdown format. The maximum message length is 7439 bytes.
    pub markdown: Option<String>,
    /// The public URL to a binary file to be posted into the room. Only one file is allowed per message. Uploaded files are automatically converted into a format that all Webex Teams clients can render. For the supported media types and the behavior of uploads, see the [Message Attachments Guide](https://developer.webex.com/docs/api/basics#message-attachments).
    pub files: Option<Vec<String>>,
    /// Content attachments to attach to the message. Only one card per message is supported.
    pub attachments: Option<Vec<Attachment>>,
}

/// Webex Teams message information.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    /// The unique identifier for the message.
    pub id: Option<String>,
    /// The room ID of the message.
    pub room_id: Option<String>,
    /// The room type.
    pub room_type: Option<RoomType>,
    /// The person ID of the recipient when sending a private 1:1 message.
    pub to_person_id: Option<String>,
    /// The email address of the recipient when sending a private 1:1 message.
    pub to_person_email: Option<String>,
    /// The message, in plain text. If markdown is specified this parameter may be optionally used to provide alternate text for UI clients that do not support rich text.
    pub text: Option<String>,
    /// The message, in Markdown format.
    pub markdown: Option<String>,
    /// The text content of the message, in HTML format. This read-only property is used by the Webex Teams clients.
    pub html: Option<String>,
    /// Public URLs for files attached to the message. For the supported media types and the behavior of file uploads, see Message Attachments.
    pub files: Option<Vec<String>>,
    /// The person ID of the message author.
    pub person_id: Option<String>,
    /// The email address of the message author.
    pub person_email: Option<String>,
    /// People IDs for anyone mentioned in the message.
    pub mentioned_people: Option<Vec<String>>,
    /// Group names for the groups mentioned in the message.
    pub mentioned_groups: Option<Vec<String>>,
    /// Message content attachments attached to the message.
    pub attachments: Option<Vec<Attachment>>,
    /// The date and time the message was created.
    pub created: Option<String>,
    /// The date and time the message was updated, if it was edited.
    pub updated: Option<String>,
    /// The ID of the "parent" message (the start of the reply chain)
    pub parent_id: Option<String>,
}

/// Parameters for listing messages.
#[skip_serializing_none]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageListParams<'a> {
    /// List messages in a room, by ID.
    pub room_id: &'a str,
    /// List messages with a parent, by ID.
    pub parent_id: Option<&'a str>,
    /// List messages with these people mentioned, by ID. Use me as a shorthand for the current API user.
    /// Only me or the person ID of the current user may be specified. Bots must include this parameter
    /// to list messages in group rooms (spaces).
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    pub mentioned_people: &'a [&'a str],
    /// List messages sent before a date and time.
    pub before: Option<&'a str>,
    /// List messages sent before a message, by ID.
    pub before_message: Option<&'a str>,
    /// Limit the maximum number of messages in the response.
    /// Default: 50
    pub max: Option<u32>,
}

impl<'a> MessageListParams<'a> {
    /// Creates a new `MessageListParams` with the given room ID.
    #[allow(clippy::must_use_candidate)]
    pub const fn new(room_id: &'a str) -> Self {
        Self {
            room_id,
            parent_id: None,
            mentioned_people: &[],
            before: None,
            before_message: None,
            max: None,
        }
    }
}

/// Parameters for editing a message.
/// `room_id` is required, and at least one of `text` or `markdown` must be set.
/// Follows <https://developer.webex.com/docs/api/v1/messages/edit-a-message>
#[skip_serializing_none]
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageEditParams<'a> {
    /// The id of the room the message is posted in.
    pub room_id: &'a str,
    /// The plain text content of the message. If markdown is specified this parameter may be optionally
    /// used to provide alternate text for UI clients that do not support rich text.
    pub text: Option<&'a str>,
    /// The markdown content of the message. If this attribute is set ensure that the request does NOT contain an html attribute.
    pub markdown: Option<&'a str>,
    /// The message, in HTML format. The maximum message length is 7439 bytes.
    pub html: Option<&'a str>,
}
