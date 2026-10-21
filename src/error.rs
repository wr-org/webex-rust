use reqwest::StatusCode;

/// Errors that can occur when using the Webex API client.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    // Foreign errors
    /// IO error from standard library operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error.
    ///
    /// Occurs when parsing API responses or serializing request bodies.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::error::Error),

    /// URL form encoding error when serializing query parameters.
    #[error("URL form encoding error: {0}")]
    FormEncoding(#[from] serde_html_form::ser::Error),

    /// UTF-8 decoding error.
    #[error("UTF8 error: {0}")]
    UTF8(#[from] std::str::Utf8Error),

    /// HTTP client error from reqwest.
    ///
    /// Wraps errors from the underlying HTTP client, including network errors,
    /// connection failures, and timeout errors.
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    // WS/request errors
    /// WebSocket connection was closed.
    ///
    /// The WebSocket connection to the Webex event stream was closed,
    /// either by the server or due to a network error.
    #[error("Connection was closed: {0}")]
    Closed(String),

    /// HTTP error status code without detailed message.
    ///
    /// The API returned an HTTP error status code (4xx or 5xx).
    /// Common codes:
    /// - 401: Unauthorized (invalid or expired token)
    /// - 403: Forbidden (missing OAuth scopes)
    /// - 404: Not Found (resource doesn't exist)
    /// - 429: Too Many Requests (rate limited)
    /// - 500: Internal Server Error
    #[error("HTTP Status: '{0}'")]
    Status(StatusCode),

    /// HTTP error status code with detailed error message.
    ///
    /// Like [`Status`](Error::Status), but includes the error message from the API response.
    #[error("HTTP Status: '{0}' Message: {1}")]
    StatusText(StatusCode, String),

    /// Rate limiting error with optional retry delay.
    ///
    /// The API returned HTTP 423 (Locked) or 429 (Too Many Requests). The second
    /// field contains a nonnegative Retry-After delay in seconds, if provided.
    #[error("{0} Retry in: '{1:?}'")]
    Limited(StatusCode, Option<i64>),

    /// WebSocket protocol error from tungstenite.
    ///
    /// Errors from the underlying WebSocket implementation, such as protocol
    /// violations, handshake failures, or frame parsing errors.
    #[error("{0} {1}")]
    Tungstenite(Box<tokio_tungstenite::tungstenite::Error>, String),

    /// Webex API behavior changed unexpectedly.
    ///
    /// The API response format or behavior differs from what this library expects.
    /// This usually indicates that Cisco changed the API in a backwards-incompatible way.
    #[error("Webex API changed: {0}")]
    Api(&'static str),

    /// Authentication or authorization error.
    ///
    /// Generic authentication failure, typically when the token is invalid
    /// or missing required permissions.
    #[error("Authentication error")]
    Authentication,

    /// User-facing error message.
    ///
    /// Error created from application logic with a custom message intended
    /// for end users.
    #[error("{0}")]
    UserError(String),

    // catch-all
    /// Unknown or uncategorized error.
    ///
    /// Fallback error type for errors that don't fit other categories.
    #[error("Unknown error: {0}")]
    Other(String),
}

/// Errors from the explicit page and complete-enumeration APIs.
///
/// Request failures retain the original [`Error`] as their source, including
/// [`Error::Limited`] for HTTP 423/429. Pagination-only failures are separate so
/// the existing exhaustive [`Error`] enum stays source-compatible.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum PaginationError {
    /// Request, response decoding, or query serialization failed.
    #[error("{0}")]
    Request(#[from] Error),

    /// A malformed, ambiguous, or unsafe pagination link or redirect.
    #[error("Invalid pagination continuation: {0}")]
    InvalidContinuation(String),

    /// Complete enumeration needs another page after exhausting its budget.
    #[error("Pagination page budget exhausted ({max_pages} pages)")]
    Limit {
        /// Configured maximum page count, including the first page.
        max_pages: usize,
    },

    /// Complete enumeration encountered a previously requested continuation.
    #[error("Pagination continuation loop detected")]
    Loop,
}

impl PaginationError {
    /// Preserve the legacy room inventory return type without losing request errors.
    pub(crate) fn into_legacy_error(self) -> Error {
        match self {
            Self::Request(error) => error,
            other => Error::Other(other.to_string()),
        }
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Other(s)
    }
}
impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Error::Other(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pagination_request_keeps_original_error_as_source() {
        use std::error::Error as _;
        let error = PaginationError::from(Error::Limited(StatusCode::TOO_MANY_REQUESTS, Some(12)));
        assert!(matches!(
            error.source().unwrap().downcast_ref::<Error>(),
            Some(Error::Limited(StatusCode::TOO_MANY_REQUESTS, Some(12)))
        ));
        assert_eq!(
            error.to_string(),
            Error::Limited(StatusCode::TOO_MANY_REQUESTS, Some(12)).to_string()
        );
    }

    #[test]
    fn legacy_inventory_mapping_preserves_requests_and_describes_local_failures() {
        let request = PaginationError::from(Error::Limited(StatusCode::LOCKED, Some(12)));
        assert!(matches!(
            request.into_legacy_error(),
            Error::Limited(StatusCode::LOCKED, Some(12))
        ));
        for error in [
            PaginationError::Limit { max_pages: 1 },
            PaginationError::Loop,
            PaginationError::InvalidContinuation("unsafe destination".into()),
        ] {
            let expected = error.to_string();
            assert!(
                matches!(error.into_legacy_error(), Error::Other(message) if message == expected)
            );
        }
    }

    #[test]
    fn test_error_from_string() {
        let error: Error = "test error".to_string().into();
        assert!(matches!(error, Error::Other(_)));
        assert_eq!(error.to_string(), "Unknown error: test error");
    }

    #[test]
    fn test_error_from_str() {
        let error: Error = "test error".into();
        assert!(matches!(error, Error::Other(_)));
        assert_eq!(error.to_string(), "Unknown error: test error");
    }

    #[test]
    fn test_error_closed() {
        let error = Error::Closed("connection lost".to_string());
        assert_eq!(error.to_string(), "Connection was closed: connection lost");
    }

    #[test]
    fn test_error_status() {
        let error = Error::Status(StatusCode::NOT_FOUND);
        assert_eq!(error.to_string(), "HTTP Status: '404 Not Found'");
    }

    #[test]
    fn test_error_status_text() {
        let error = Error::StatusText(StatusCode::FORBIDDEN, "Missing scopes".to_string());
        assert_eq!(
            error.to_string(),
            "HTTP Status: '403 Forbidden' Message: Missing scopes"
        );
    }

    #[test]
    fn test_error_limited_with_retry() {
        let error = Error::Limited(StatusCode::TOO_MANY_REQUESTS, Some(60));
        assert!(error.to_string().contains("429"));
        assert!(error.to_string().contains("60"));
    }

    #[test]
    fn test_error_limited_without_retry() {
        let error = Error::Limited(StatusCode::TOO_MANY_REQUESTS, None);
        assert!(error.to_string().contains("429"));
    }

    #[test]
    fn test_error_api() {
        let error = Error::Api("unexpected response format");
        assert_eq!(
            error.to_string(),
            "Webex API changed: unexpected response format"
        );
    }

    #[test]
    fn test_error_authentication() {
        let error = Error::Authentication;
        assert_eq!(error.to_string(), "Authentication error");
    }

    #[test]
    fn test_error_user_error() {
        let error = Error::UserError("Invalid input provided".to_string());
        assert_eq!(error.to_string(), "Invalid input provided");
    }
}
