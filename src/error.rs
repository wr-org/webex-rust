use reqwest::StatusCode;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // Foreign errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::error::Error),
    #[error("URL form encoding error: {0}")]
    FormEncoding(#[from] serde_html_form::ser::Error),
    #[error("UTF8 error: {0}")]
    UTF8(#[from] std::str::Utf8Error),

    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    // WS/request errors
    #[error("Connection was closed: {0}")]
    Closed(String),
    #[error("HTTP Status: '{0}'")]
    Status(StatusCode),
    #[error("HTTP Status: '{0}' Message: {1}")]
    StatusText(StatusCode, String),
    #[error("{0} Retry in: '{1:?}'")]
    Limited(StatusCode, Option<i64>),
    #[error("{0} {1}")]
    Tungstenite(tokio_tungstenite::tungstenite::Error, String),
    #[error("Webex API changed: {0}")]
    Api(&'static str),

    #[error("Authentication error")]
    Authentication,

    // catch-all
    #[error("Unknown error: {0}")]
    Other(String),
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
