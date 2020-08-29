# webex-rust

[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

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

# DISCLAIMER

This crate is not maintained by Cisco, and not an official SDK.  The
authors are current developers at Cisco, but have no direct affiliation
with the Webex development team.

## License

webex-rust is provided under the MIT license. See [LICENSE](LICENSE).
