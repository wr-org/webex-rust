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

## List pagination

`list` and `list_with_params` return one page, preserving existing behavior.
For caller-controlled pagination, fetch a `Page<T>` and use its opaque
continuation when ready. Empty pages can still have a continuation:

```rust,no_run
# async fn example(webex: &webex::Webex) -> Result<(), webex::error::Error> {
let mut page = webex.list_page::<webex::Room>().await?;
loop {
    for room in page.items { println!("{:?}", room.title); }
    // Stop here to keep the continuation for later, or request the next page.
    let Some(next) = page.continuation else { break };
    page = webex.list_next_page(&next).await?;
}
# Ok(())
# }
```

For complete enumeration, select an explicit page budget:

```rust,no_run
# async fn example(webex: &webex::Webex) -> Result<(), webex::error::Error> {
let rooms = webex.list_all_with_params::<webex::Room>(
    webex::RoomListParams { max: Some(100), ..Default::default() },
    webex::PaginationOptions { max_pages: 2_000 },
).await?;
# Ok(())
# }
```

`list_page_with_params` accepts the same resource parameters; `list_all` omits
filters. `max` always controls Webex's page size, not the total results. Server
continuations retain their own cursors and filters. All-pages calls return
`PaginationLimit` when the budget is exhausted, `PaginationLoop` for repeated
continuations, and `InvalidPagination` for malformed or unsafe links. They do
not return partial results or retry errors; manual pagination lets callers
retain progress and choose their own stopping/retry policy.

`get_all_rooms` enumerates rooms, teams, and each team's rooms with a default
budget of 1,000 pages per list. Use `get_all_rooms_with_options` to change it.
Credentials are sent only to the configured origin or the public HTTPS
`api.ciscospark.com`/`webexapis.com` aliases; redirects use the same validation.

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
