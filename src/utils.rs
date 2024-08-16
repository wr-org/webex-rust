//! Utilities for webex-rs.

use super::{Body, Error};
use http_body_util::BodyExt;

/// Serialize a data structure to a JSON body.
pub(crate) fn serialize_to_body<D>(data: &D) -> Result<Body, Error>
where
    D: serde::Serialize,
{
    let json = serde_json::to_string(data)?;
    Ok(http_body_util::Full::new(json.into())
        .map_err(|_| unreachable!())
        .boxed())
}
