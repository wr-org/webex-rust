//! Low-level REST client for Webex API requests.

use crate::error::Error;
use crate::types::{Continuation, Gettable, ListResult, Page, PaginationOptions};
use log::{error, trace};
use reqwest::StatusCode;
use serde::{de::DeserializeOwned, Serialize};
use serde_json;
use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
    sync::OnceLock,
};

mod pagination;
use pagination::{next_link, validate_destination};

/// Authorization type for REST requests.
#[derive(Clone, Copy)]
pub enum AuthorizationType<'a> {
    /// No authorization
    None,
    /// Bearer token authorization
    Bearer(&'a str),
    /// Basic authentication
    Basic {
        /// Username
        username: &'a str,
        /// Password
        password: &'a str,
    },
}

/// Body type for REST requests.
enum Body<T: Serialize> {
    Json(T),
    UrlEncoded(T),
}

const BODY_NONE: Option<Body<()>> = None;

/// Implements low level REST requests to be used internally by the library.
#[derive(Clone)]
pub struct RestClient {
    /// Host prefix mapping for different API endpoints
    pub host_prefix: HashMap<String, String>,
    /// Underlying HTTP client
    pub web_client: reqwest::Client,
}

impl RestClient {
    /// Creates a new `RestClient`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            host_prefix: HashMap::new(),
            web_client: reqwest::Client::new(),
        }
    }

    /// Creates a `RestClient` with existing `host_prefix` and `web_client`.
    #[must_use]
    pub const fn new_with(
        host_prefix: HashMap<String, String>,
        web_client: reqwest::Client,
    ) -> Self {
        Self {
            host_prefix,
            web_client,
        }
    }

    /// Performs a GET request and returns the resource as JSON.
    ///
    /// # Arguments
    ///
    /// * `token` - Authorization token
    /// * `full_url` - Full URL to GET (e.g. `<https://api.ciscospark.com/v1/messages/abc123`>)
    pub async fn get_full_url<R: DeserializeOwned>(
        &self,
        token: &str,
        full_url: &str,
    ) -> Result<R, Error> {
        self.get_with_url(token, full_url).await
    }

    /// Performs a GET request with custom headers and returns the resource as JSON.
    ///
    /// # Arguments
    ///
    /// * `token` - Authorization token
    /// * `url` - Resource path (e.g. "rooms")
    /// * `params` - Query parameters
    /// * `host` - Optional custom host prefix
    pub async fn get_with_params<R: DeserializeOwned, P: Serialize>(
        &self,
        token: &str,
        url: &str,
        params: &P,
        host: Option<&str>,
    ) -> Result<R, Error> {
        let host_prefix = self.get_host_prefix(host, "rest");
        let full_url = format!("{host_prefix}/{url}");
        self.request_with_query(
            "GET",
            AuthorizationType::Bearer(token),
            &full_url,
            Some(params),
            BODY_NONE,
        )
        .await
    }

    /// Performs a GET request.
    async fn get_with_url<R: DeserializeOwned>(
        &self,
        token: &str,
        full_url: &str,
    ) -> Result<R, Error> {
        self.request("GET", AuthorizationType::Bearer(token), full_url, BODY_NONE)
            .await
    }

    /// Performs a POST request with JSON body.
    pub async fn post<R: DeserializeOwned, D: Serialize>(
        &self,
        token: &str,
        url: &str,
        data: &D,
        host: Option<&str>,
    ) -> Result<R, Error> {
        let host_prefix = self.get_host_prefix(host, "rest");
        let full_url = format!("{host_prefix}/{url}");
        self.request(
            "POST",
            AuthorizationType::Bearer(token),
            &full_url,
            Some(Body::Json(data)),
        )
        .await
    }

    /// Performs a PUT request with JSON body.
    pub async fn put<R: DeserializeOwned, D: Serialize>(
        &self,
        token: &str,
        url: &str,
        data: &D,
        host: Option<&str>,
    ) -> Result<R, Error> {
        let host_prefix = self.get_host_prefix(host, "rest");
        let full_url = format!("{host_prefix}/{url}");
        self.request(
            "PUT",
            AuthorizationType::Bearer(token),
            &full_url,
            Some(Body::Json(data)),
        )
        .await
    }

    /// Performs a DELETE request.
    pub async fn delete<T: Gettable>(
        &self,
        token: &str,
        id: &str,
        host: Option<&str>,
    ) -> Result<(), Error> {
        let host_prefix = self.get_host_prefix(host, "rest");
        let full_url = format!("{host_prefix}/{}/{id}", T::API_ENDPOINT);
        self.send_request(
            "DELETE",
            AuthorizationType::Bearer(token),
            &full_url,
            None::<&()>,
            BODY_NONE,
        )
        .await?;
        Ok(())
    }

    /// Performs a POST request with URL-encoded form body.
    /// Used primarily for OAuth authentication flows.
    pub async fn api_post_form_urlencoded<T: DeserializeOwned, B: Serialize>(
        &self,
        rest_method: &str,
        body: B,
        params: Option<impl Serialize>,
        auth: AuthorizationType<'_>,
    ) -> Result<T, Error> {
        // Get the host prefix for the URL
        let url_trimmed = rest_method.split('?').next().unwrap_or(rest_method);
        let prefix = self
            .host_prefix
            .get(url_trimmed)
            .map_or(super::REST_HOST_PREFIX, String::as_str);
        let full_url = format!("{prefix}/{rest_method}");

        self.request_with_query(
            "POST",
            auth,
            &full_url,
            params.as_ref(),
            Some(Body::UrlEncoded(body)),
        )
        .await
    }

    /// Gets the host prefix for a given host key.
    fn get_host_prefix(&self, host: Option<&str>, default_key: &str) -> String {
        host.map_or_else(
            || {
                self.host_prefix
                    .get(default_key)
                    .map_or_else(|| super::REST_HOST_PREFIX.to_string(), Clone::clone)
            },
            ToString::to_string,
        )
    }

    /// Performs an HTTP request with query parameters and optional body.
    async fn request_with_query<R: DeserializeOwned, Q: Serialize, D: Serialize>(
        &self,
        method: &str,
        auth: AuthorizationType<'_>,
        url: &str,
        query: Option<&Q>,
        body: Option<Body<D>>,
    ) -> Result<R, Error> {
        let response = self.send_request(method, auth, url, query, body).await?;
        Self::decode_response(response).await
    }

    /// Decodes a successful response, preserving the shared empty-body behavior.
    async fn decode_response<R: DeserializeOwned>(response: reqwest::Response) -> Result<R, Error> {
        let response_text = response.text().await?;
        trace!("Response: {response_text}");
        if response_text.is_empty() {
            Ok(serde_json::from_str("{}")?)
        } else {
            Ok(serde_json::from_str(&response_text)?)
        }
    }

    /// Performs an HTTP request with the given method, URL, and optional body.
    async fn request<R: DeserializeOwned, D: Serialize>(
        &self,
        method: &str,
        auth: AuthorizationType<'_>,
        url: &str,
        body: Option<Body<D>>,
    ) -> Result<R, Error> {
        self.request_with_query(method, auth, url, None::<&()>, body)
            .await
    }

    /// Sends a request and checks its status before decoding any response body.
    async fn send_request<Q: Serialize, D: Serialize>(
        &self,
        method: &str,
        auth: AuthorizationType<'_>,
        url: &str,
        query: Option<&Q>,
        body: Option<Body<D>>,
    ) -> Result<reqwest::Response, Error> {
        let response =
            Self::send_request_with_client(&self.web_client, method, auth, url, query, body)
                .await?;
        Self::check_response(response, url).await
    }

    async fn send_request_with_client<Q: Serialize, D: Serialize>(
        client: &reqwest::Client,
        method: &str,
        auth: AuthorizationType<'_>,
        url: &str,
        query: Option<&Q>,
        body: Option<Body<D>>,
    ) -> Result<reqwest::Response, Error> {
        trace!("{method} {url}");
        let mut req = client
            .request(method.parse().unwrap(), url)
            .header("User-Agent", format!("webex-rust/{}", super::CRATE_VERSION));
        if let Some(params) = query {
            req = req.query(params);
        }
        req = match auth {
            AuthorizationType::None => req,
            AuthorizationType::Bearer(token) => req.bearer_auth(token),
            AuthorizationType::Basic { username, password } => {
                req.basic_auth(username, Some(password))
            }
        };
        req = match body {
            Some(Body::Json(data)) => req.json(&data),
            Some(Body::UrlEncoded(data)) => req.form(&data),
            None => req,
        };
        Ok(req.send().await?)
    }

    async fn check_response(
        response: reqwest::Response,
        url: &str,
    ) -> Result<reqwest::Response, Error> {
        let status = response.status();
        if status == StatusCode::LOCKED || status == StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<i64>().ok())
                .filter(|seconds| *seconds >= 0);
            return Err(Error::Limited(status, retry_after));
        }
        if status.is_success() {
            Ok(response)
        } else {
            let response_text = response.text().await?;
            Self::log_error(status, url, &response_text);
            Err(Self::handle_error_response(status, response_text))
        }
    }

    /// Logs HTTP errors with appropriate log level based on context.
    fn log_error(status: StatusCode, url: &str, response_text: &str) {
        // Try to parse as JSON to get structured error message
        if let Ok(json_error) = serde_json::from_str::<serde_json::Value>(response_text) {
            if let Some(message) = json_error.get("message").and_then(|m| m.as_str()) {
                // Team 404 errors are expected when user doesn't have team access - log as debug
                if status == StatusCode::NOT_FOUND
                    && url.contains("/teams")
                    && message.contains("Could not find teams")
                {
                    trace!("HTTP {status} for {url}: {message} (expected when not a team member)");
                    return;
                }
            }
        }

        // Log all other errors at error level
        error!("HTTP {status}: {response_text}");
    }

    /// Handles error responses from the API.
    fn handle_error_response(status: StatusCode, response_text: String) -> Error {
        let html = response_text.trim_start();
        if html
            .get(..14)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("<!doctype html"))
            || html
                .get(..5)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("<html"))
        {
            let title = extract_html_title(&response_text, status);
            Error::StatusText(status, title)
        } else {
            match serde_json::from_str::<serde_json::Value>(&response_text) {
                Ok(json) => {
                    if let Some(message) = json.get("message").and_then(|v| v.as_str()) {
                        Error::StatusText(status, message.to_string())
                    } else {
                        Error::StatusText(status, response_text)
                    }
                }
                Err(_) => Error::StatusText(status, response_text),
            }
        }
    }

    /// Generic GET request for any `Gettable` type.
    pub async fn get<T: Gettable + DeserializeOwned>(
        &self,
        token: &str,
        id: &str,
        host: Option<&str>,
    ) -> Result<T, Error> {
        let host_prefix = self.get_host_prefix(host, "rest");
        let full_url = format!("{host_prefix}/{}/{id}", T::API_ENDPOINT);
        self.get_with_url(token, &full_url).await
    }

    /// Generic LIST request for any `Gettable` type.
    pub async fn list<T: Gettable + DeserializeOwned>(
        &self,
        token: &str,
        params: &T::ListParams<'_>,
        host: Option<&str>,
    ) -> Result<Vec<T>, Error> {
        let list_result: ListResult<T> = self
            .get_with_params(token, T::API_ENDPOINT, params, host)
            .await?;

        // Handle both 'items' and 'devices' fields
        Ok(list_result
            .items
            .or(list_result.devices)
            .unwrap_or_default())
    }

    /// Fetch a first page with parameters applied exactly once.
    pub(super) async fn api_get_page<T: Gettable + DeserializeOwned>(
        &self,
        params: Option<impl Serialize>,
        auth: AuthorizationType<'_>,
    ) -> Result<Page<T>, Error> {
        let origin = self.pagination_origin::<T>()?;
        let mut url = origin.clone();
        if let Some(params) = params {
            let query = serde_html_form::to_string(params)?;
            if !query.is_empty() {
                url.set_query(Some(&query));
            }
        }
        self.fetch_page(url, origin, auth).await
    }

    fn pagination_origin<T: Gettable>(&self) -> Result<url::Url, Error> {
        let prefix = self
            .host_prefix
            .get(T::API_ENDPOINT)
            .map_or(super::REST_HOST_PREFIX, String::as_str);
        url::Url::parse(&format!("{prefix}/{}", T::API_ENDPOINT))
            .map_err(|_| Error::InvalidPagination("invalid API origin".into()))
    }

    pub(super) async fn api_next_page<T: Gettable + DeserializeOwned>(
        &self,
        continuation: &Continuation<T>,
        auth: AuthorizationType<'_>,
    ) -> Result<Page<T>, Error> {
        // Revalidate against this client's configuration before attaching auth.
        let origin = self.pagination_origin::<T>()?;
        if origin.origin() != continuation.origin.origin() {
            return Err(Error::InvalidPagination(
                "continuation belongs to another API origin".into(),
            ));
        }
        self.fetch_page(continuation.url.clone(), origin, auth)
            .await
    }

    async fn fetch_page<T: DeserializeOwned>(
        &self,
        mut url: url::Url,
        origin: url::Url,
        auth: AuthorizationType<'_>,
    ) -> Result<Page<T>, Error> {
        // reqwest cannot override redirect policy per request. Use a pooled,
        // no-redirect transport for pages, then validate every Location ourselves.
        // Request construction and status decoding remain shared with other REST calls.
        static PAGE_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
        let client = PAGE_CLIENT.get_or_init(|| {
            reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("failed to initialize pagination HTTP client")
        });
        let mut redirects = HashSet::new();
        loop {
            validate_destination(&origin, &url)?;
            if !redirects.insert(url.clone()) || redirects.len() > 10 {
                return Err(Error::InvalidPagination(
                    "redirect loop or redirect limit exceeded".into(),
                ));
            }
            let response = Self::send_request_with_client(
                client,
                "GET",
                auth,
                url.as_str(),
                None::<&()>,
                BODY_NONE,
            )
            .await?;
            if matches!(response.status().as_u16(), 301 | 302 | 303 | 307 | 308) {
                let location = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|value| value.to_str().ok())
                    .ok_or_else(|| {
                        Error::InvalidPagination("redirect without a valid Location".into())
                    })?;
                url = pagination::resolve_destination(&origin, &url, location)?;
                continue;
            }
            let response = Self::check_response(response, url.as_str()).await?;
            let next = next_link(response.headers(), &origin, &url)?;
            let body: ListResult<T> = Self::decode_response(response).await?;
            return Ok(Page {
                items: body.items.or(body.devices).unwrap_or_default(),
                continuation: next.map(|url| Continuation {
                    url,
                    origin,
                    resource: PhantomData,
                }),
            });
        }
    }

    /// Collect through the same first/next page implementation used manually.
    pub(super) async fn api_list_all<T: Gettable + DeserializeOwned>(
        &self,
        params: Option<impl Serialize>,
        auth: AuthorizationType<'_>,
        options: PaginationOptions,
    ) -> Result<Vec<T>, Error> {
        if options.max_pages == 0 {
            return Err(Error::PaginationLimit { max_pages: 0 });
        }
        let mut page = self.api_get_page::<T>(params, auth).await?;
        let mut items = page.items;
        let mut visited = HashSet::new();
        let mut pages = 1;
        while let Some(next) = page.continuation {
            // Check loops independently of the caller's request budget.
            if !visited.insert(next.url.clone()) {
                return Err(Error::PaginationLoop);
            }
            if pages >= options.max_pages {
                return Err(Error::PaginationLimit {
                    max_pages: options.max_pages,
                });
            }
            page = self.api_next_page(&next, auth).await?;
            items.extend(page.items);
            pages += 1;
        }
        Ok(items)
    }

    // Legacy API methods for compatibility with client/mod.rs

    /// Performs a GET request (legacy API name).
    pub async fn api_get<R: DeserializeOwned>(
        &self,
        rest_method: &str,
        params: Option<impl Serialize>,
        auth: AuthorizationType<'_>,
    ) -> Result<R, Error> {
        let url_trimmed = rest_method.split('?').next().unwrap_or(rest_method);
        let prefix = self
            .host_prefix
            .get(url_trimmed)
            .map_or(super::REST_HOST_PREFIX, String::as_str);
        let full_url = format!("{prefix}/{rest_method}");

        if let Some(params) = params {
            self.request_with_query("GET", auth, &full_url, Some(&params), BODY_NONE)
                .await
        } else {
            self.request("GET", auth, &full_url, BODY_NONE).await
        }
    }

    /// Performs a POST request with JSON body (legacy API name).
    pub async fn api_post<R: DeserializeOwned>(
        &self,
        rest_method: &str,
        body: impl Serialize,
        params: Option<impl Serialize>,
        auth: AuthorizationType<'_>,
    ) -> Result<R, Error> {
        let url_trimmed = rest_method.split('?').next().unwrap_or(rest_method);
        let prefix = self
            .host_prefix
            .get(url_trimmed)
            .map_or(super::REST_HOST_PREFIX, String::as_str);
        let full_url = format!("{prefix}/{rest_method}");

        self.request_with_query(
            "POST",
            auth,
            &full_url,
            params.as_ref(),
            Some(Body::Json(body)),
        )
        .await
    }

    /// Performs a PUT request with JSON body (legacy API name).
    pub async fn api_put<R: DeserializeOwned>(
        &self,
        rest_method: &str,
        body: impl Serialize,
        params: Option<impl Serialize>,
        auth: AuthorizationType<'_>,
    ) -> Result<R, Error> {
        let url_trimmed = rest_method.split('?').next().unwrap_or(rest_method);
        let prefix = self
            .host_prefix
            .get(url_trimmed)
            .map_or(super::REST_HOST_PREFIX, String::as_str);
        let full_url = format!("{prefix}/{rest_method}");

        self.request_with_query(
            "PUT",
            auth,
            &full_url,
            params.as_ref(),
            Some(Body::Json(body)),
        )
        .await
    }

    /// Performs a DELETE request (legacy API name).
    pub async fn api_delete(
        &self,
        rest_method: &str,
        params: Option<impl Serialize>,
        auth: AuthorizationType<'_>,
    ) -> Result<(), Error> {
        let url_trimmed = rest_method.split('?').next().unwrap_or(rest_method);
        let prefix = self
            .host_prefix
            .get(url_trimmed)
            .map_or(super::REST_HOST_PREFIX, String::as_str);
        let full_url = format!("{prefix}/{rest_method}");

        self.send_request("DELETE", auth, &full_url, params.as_ref(), BODY_NONE)
            .await?;
        Ok(())
    }
}

impl Default for RestClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract title from HTML error page.
fn extract_html_title(html: &str, status: StatusCode) -> String {
    if let (Some(start_pos), Some(end_pos)) = (html.find("<title>"), html.find("</title>")) {
        let start = start_pos + 7;
        if start < end_pos && end_pos <= html.len() {
            return html[start..end_pos].to_string();
        }
    }
    format!("HTTP {} - HTML error page returned", status.as_u16())
}

#[cfg(test)]
// Server guards must stay alive until all requests and mock assertions finish.
#[allow(clippy::significant_drop_tightening)]
mod tests {
    use super::*;
    use mockito::Matcher;
    use serde_json::json;

    fn client(server: &mockito::Server) -> RestClient {
        RestClient::new_with(
            HashMap::from([("resource".to_string(), server.url())]),
            reqwest::Client::new(),
        )
    }

    #[tokio::test]
    async fn non_get_requests_preserve_query_parameters() {
        let mut server = mockito::Server::new_async().await;
        let client = client(&server);
        let params = [("filter", "a & b")];
        for method in ["POST", "PUT", "DELETE", "FORM"] {
            let mock = server
                .mock(if method == "FORM" { "POST" } else { method }, "/resource")
                .match_query(Matcher::AllOf(vec![
                    Matcher::UrlEncoded("existing".into(), "yes".into()),
                    Matcher::UrlEncoded("filter".into(), "a & b".into()),
                ]))
                .match_header("authorization", "Bearer test-token")
                .match_header(
                    "content-type",
                    match method {
                        "FORM" => Matcher::Exact("application/x-www-form-urlencoded".into()),
                        "DELETE" => Matcher::Missing,
                        _ => Matcher::Exact("application/json".into()),
                    },
                )
                .match_body(match method {
                    "FORM" => Matcher::UrlEncoded("field".into(), "value".into()),
                    "DELETE" => Matcher::Any,
                    _ => Matcher::Json(json!({"field": "value"})),
                })
                .with_status(200)
                .with_body("{}")
                .create_async()
                .await;
            let url = "resource?existing=yes";
            let auth = AuthorizationType::Bearer("test-token");
            let result: Result<serde_json::Value, Error> = match method {
                "POST" => {
                    client
                        .api_post(url, json!({"field": "value"}), Some(params), auth)
                        .await
                }
                "PUT" => {
                    client
                        .api_put(url, json!({"field": "value"}), Some(params), auth)
                        .await
                }
                "FORM" => {
                    client
                        .api_post_form_urlencoded(url, [("field", "value")], Some(params), auth)
                        .await
                }
                _ => client
                    .api_delete(url, Some(params), auth)
                    .await
                    .map(|()| json!({})),
            };
            assert!(result.is_ok(), "{method}: {result:?}");
            mock.assert_async().await;
        }
    }

    #[tokio::test]
    async fn delete_accepts_successful_non_json_responses() {
        let mut server = mockito::Server::new_async().await;
        let client = client(&server);
        for (status, body) in [(200, "deleted"), (202, "accepted"), (204, "")] {
            let mock = server
                .mock("DELETE", "/resource")
                .with_status(status)
                .with_body(body)
                .create_async()
                .await;
            client
                .api_delete("resource", None::<()>, AuthorizationType::None)
                .await
                .unwrap();
            mock.assert_async().await;
        }
        let mock = server
            .mock("DELETE", "/rooms/room-id")
            .with_status(200)
            .with_body("deleted")
            .create_async()
            .await;
        client
            .delete::<crate::Room>("token", "room-id", Some(&server.url()))
            .await
            .unwrap();
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn rate_limits_preserve_status_and_retry_delay() {
        let mut server = mockito::Server::new_async().await;
        let client = client(&server);
        for method in ["GET", "DELETE"] {
            for status in [423, 429] {
                for (header, expected) in [
                    (Some("60"), Some(60)),
                    (Some("0"), Some(0)),
                    (None, None),
                    (Some("invalid"), None),
                    (Some("-1"), None),
                ] {
                    let mut mock = server
                        .mock(method, "/resource")
                        .with_status(status)
                        .with_body(r#"{"message":"Try later"}"#);
                    if let Some(header) = header {
                        mock = mock.with_header("retry-after", header);
                    }
                    let mock = mock.create_async().await;
                    let error = if method == "GET" {
                        client
                            .api_get::<serde_json::Value>(
                                "resource",
                                None::<()>,
                                AuthorizationType::None,
                            )
                            .await
                            .unwrap_err()
                    } else {
                        client
                            .api_delete("resource", None::<()>, AuthorizationType::None)
                            .await
                            .unwrap_err()
                    };
                    assert!(
                        matches!(error, Error::Limited(code, delay) if usize::from(code.as_u16()) == status && delay == expected),
                        "{error:?}"
                    );
                    mock.assert_async().await;
                }
            }
        }
    }

    #[tokio::test]
    async fn errors_preserve_json_messages_and_html_titles() {
        let mut server = mockito::Server::new_async().await;
        let client = client(&server);
        for (body, expected) in [
            (r#"{"message":"Missing scope"}"#, "Missing scope"),
            (
                "<!doctype html><html><title>Forbidden</title></html>",
                "Forbidden",
            ),
            (
                "  <!DOCTYPE html><html><title>Forbidden</title></html>",
                "Forbidden",
            ),
        ] {
            let mock = server
                .mock("GET", "/resource")
                .with_status(403)
                .with_body(body)
                .create_async()
                .await;
            let error = client
                .api_get::<serde_json::Value>("resource", None::<()>, AuthorizationType::None)
                .await
                .unwrap_err();
            assert!(
                matches!(error, Error::StatusText(StatusCode::FORBIDDEN, ref message) if message == expected),
                "{error:?}"
            );
            mock.assert_async().await;
        }
    }

    #[tokio::test]
    async fn pagination_preserves_errors_on_later_pages() {
        let mut server = mockito::Server::new_async().await;
        let client = RestClient::new_with(
            HashMap::from([("rooms".to_string(), server.url())]),
            reqwest::Client::new(),
        );
        for status in [403, 423, 429] {
            let next_url = format!("{}/rooms?cursor={status}", server.url());
            let first = server
                .mock("GET", "/rooms")
                .match_query(Matcher::UrlEncoded("teamId".into(), "T1".into()))
                .with_header("Link", &format!("<{next_url}>; rel=\"next\""))
                .with_body(r#"{"items":[]}"#)
                .create_async()
                .await;
            let second = server
                .mock("GET", format!("/rooms?cursor={status}").as_str())
                .match_header("authorization", "Bearer test-token")
                .match_header(
                    "user-agent",
                    format!("webex-rust/{}", super::super::CRATE_VERSION).as_str(),
                )
                .with_status(status)
                .with_header("retry-after", "12")
                .with_body(r#"{"message":"Try later"}"#)
                .create_async()
                .await;
            let error = client
                .api_list_all::<crate::Room>(
                    Some([("teamId", "T1")]),
                    AuthorizationType::Bearer("test-token"),
                    PaginationOptions::default(),
                )
                .await
                .unwrap_err();
            match error {
                Error::StatusText(code, message) if status == 403 => {
                    assert_eq!(usize::from(code.as_u16()), status);
                    assert_eq!(message, "Try later");
                }
                Error::Limited(code, delay) => {
                    assert_eq!(usize::from(code.as_u16()), status);
                    assert_eq!(delay, Some(12));
                }
                other => panic!("unexpected pagination error: {other:?}"),
            }
            first.assert_async().await;
            second.assert_async().await;
        }
    }
}
