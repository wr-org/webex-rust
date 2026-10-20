# webex-rust

[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![Crates.io](https://img.shields.io/crates/v/webex)](https://crates.io/crates/webex)
[![docs.rs](https://img.shields.io/docsrs/webex)](https://docs.rs/webex)

A minimal asynchronous interface to Webex Teams, intended for (but not
		limited to) implementing bots.

Current functionality includes:

- Registration with Webex APIs
- Monitoring an event stream
- Sending direct or group messages
- Getting room memberships
- Building AdaptiveCards and retrieving responses

Not all features are fully-fleshed out, particularly the AdaptiveCard
support (only a few serializations exist, enough to create a form with a
		few choices, a text box, and a submit button).

## Upgrading to 0.11

Client and resource types remain available through their existing imports. Two
changes can require updates in code that uses lower-level types:

- When constructing `CardElement::ActionSet` directly, include the new
  `separator` and `spacing` fields. The
  `CardElement::action_set()` builder supplies defaults for these fields.
- `WStream` and `Error::Tungstenite` now expose tungstenite 0.28 types. Update
  direct WebSocket integrations to use the matching dependency version.

New cards, and cards deserialized without a version, default to schema 1.2 so
they can contain `ActionSet` elements. Explicit versions are preserved; callers
that set a lower version must use elements supported by that version. Webex
supports cards through version 1.3.

Text alignment now serializes as `horizontalAlignment`; the old
`HorizontalAlignment` spelling is still accepted on input. `ActionSet` does not
expose alignment because that property is absent from its published schema.

# DISCLAIMER

This crate is not maintained by Cisco, and not an official SDK.  The
authors are current developers at Cisco, but have no direct affiliation
with the Webex development team.

## License

webex-rust is provided under the MIT license. See [LICENSE](LICENSE).
