#![deny(missing_docs)]
#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
// clippy::use_self fixed in https://github.com/rust-lang/rust-clippy/pull/9454
// TODO: remove this when clippy bug fixed in stable
#![allow(clippy::use_self)]
// should support this in the future - would be nice if all futures were send
#![allow(clippy::future_not_send)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::option_if_let_else)]
#![cfg_attr(test, deny(warnings))]
#![doc(html_root_url = "https://docs.rs/webex/latest/webex/")]

//! # webex-rust
//!
//! A minimal asynchronous interface to Webex Teams, intended for (but not
//! limited to) implementing bots.
//!
//! Current functionality includes:
//!
//! - Registration with Webex APIs
//! - Monitoring an event stream
//! - Sending direct or group messages
//! - Getting room memberships
//! - Building `AdaptiveCards` and retrieving responses
//!
//! Not all features are fully-fleshed out, particularly the `AdaptiveCard`
//! support (only a few serializations exist, enough to create a form with a
//! few choices, a text box, and a submit button).
//!
//! # DISCLAIMER
//!
//! This crate is not maintained by Cisco, and not an official SDK.  The
//! author is a current developer at Cisco, but has no direct affiliation
//! with the Webex development team.

pub mod adaptive_card;
pub mod auth;
pub mod client;
#[allow(missing_docs)]
pub mod error;
pub mod types;

// Re-export main client types
pub use client::{WStream, Webex, WebexEventStream};
pub use types::*;

// Constants used throughout the crate
const REST_HOST_PREFIX: &str = "https://api.ciscospark.com/v1";
const U2C_HOST_PREFIX: &str = "https://u2c.wbx2.com/u2c/api/v1";
const DEFAULT_REGISTRATION_HOST_PREFIX: &str = "https://wdm-a.wbx2.com/wdm/api/v1";
const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_DEVICE_NAME: &str = "rust-client";
const DEVICE_SYSTEM_NAME: &str = "rust-spark-client";
