//! Caller-controlled list pagination.

use std::marker::PhantomData;

/// One response page. An empty `items` vector does not imply completion:
/// only `continuation == None` means there is no next page.
#[derive(Debug)]
pub struct Page<T> {
    /// Resources in server order for this page only.
    pub items: Vec<T>,
    /// Opaque, resource-typed token for [`crate::Webex::list_next_page`].
    pub continuation: Option<Continuation<T>>,
}

/// A server-provided continuation, usable with the client that obtained it.
///
/// Its URL and query parameters are preserved internally; callers must not
/// construct or modify cursors. Fetching it performs one page request, with
/// no automatic retries. The token is borrowed so callers can retry explicitly.
#[derive(Debug)]
pub struct Continuation<T> {
    pub(crate) url: url::Url,
    pub(crate) origin: url::Url,
    pub(crate) resource: PhantomData<fn() -> T>,
}

/// Request budget for each complete list enumeration.
///
/// This counts response pages (including empty pages), not resources or HTTP
/// redirects. `max` in resource parameters remains Webex's page size. All-pages
/// methods return an error rather than partial results when the budget is
/// exhausted; use manual pages to retain progress or stop early.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PaginationOptions {
    /// Maximum pages to fetch, including the first. Zero permits no requests.
    pub max_pages: usize,
}

impl Default for PaginationOptions {
    /// Allow up to 1,000 pages per enumeration.
    fn default() -> Self {
        Self { max_pages: 1_000 }
    }
}
