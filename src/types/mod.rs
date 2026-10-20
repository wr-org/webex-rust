#![deny(missing_docs)]
//! Basic types for Webex Teams APIs

// Submodules
mod api;
pub mod attachment;
pub mod device;
pub mod event;
pub mod membership;
pub mod message;
pub mod organization;
pub mod person;
pub mod room;

// Re-export commonly used types at the crate root
pub use attachment::{Attachment, AttachmentAction};
pub use device::{Authorization, DeviceData, DeviceError};
pub use event::{
    Activity, ActivityParent, ActivityType, Actor, AlertType, Event, EventData, GlobalId,
    GlobalIdType, MessageActivity, MiscItem, MiscItems, Object, SpaceActivity, Target,
    VectorCounters,
};
pub use membership::{Membership, MembershipListParams};
pub use message::{Message, MessageEditParams, MessageListParams, MessageOut};
pub use organization::{Catalog, Destination, Organization, Team};
pub use person::{Person, PhoneNumber};
pub use room::{Room, RoomListParams, RoomType, SortRoomsBy};

// Internal types
pub(crate) use api::{Gettable, ListResult};
pub(crate) use device::DevicesReply;
pub(crate) use organization::CatalogReply;

/// Empty reply for API endpoints that return no data.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[allow(dead_code)]
pub(crate) struct EmptyReply {}
