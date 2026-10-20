//! Low-level REST client for Webex API requests.

use crate::error::Error;
use crate::types::{Gettable, ListResult};
use log::{error, trace};
use reqwest::StatusCode;
use serde::{de::DeserializeOwned, Serialize};
use serde_json;
use std::collections::HashMap;

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
        trace!("{method} {url}");
        let mut req = self
            .web_client
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
        let response = req.send().await?;
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
}
