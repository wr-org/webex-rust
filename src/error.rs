use std::error::Error as StdError;
use hyper::StatusCode;
use std::fmt;


pub type Result<T> = std::result::Result<T, Error>;


/// The Errors that may occur when processing a `Request`.
pub struct Error {
    inner: Box<Inner>,
}

pub(crate) type BoxError = Box<dyn StdError + Send + Sync>;

struct Inner {
    kind: Kind,
    description: String,
    timeout: Option<i64>,
    source: Option<BoxError>,
}

impl Error {
    pub(crate) fn new<E>(kind: Kind, source: Option<E>) -> Error
        where
            E: Into<BoxError>,
    {
        Error {
            inner: Box::new(Inner {
                kind,
                source: source.map(Into::into),
                timeout: None,
                description: "".to_string(),
            }),
        }
    }

    pub fn status(&self) -> Option<StatusCode> {
        match self.inner.kind {
            Kind::Status(code) => Some(code),
            _ => None,
        }
    }

    pub fn timeout(&self) -> Option<i64> {
        match self.inner.kind {
            Kind::Status(_) => self.inner.timeout,
            _ => None,
        }
    }

    pub fn is_timeout(&self) -> bool {
        match self.inner.timeout {
            None => { false }
            Some(_) => { true }
        }
    }

    pub(crate) fn with_timeout(mut self, timeout: Option<i64>) -> Error {
        self.inner.timeout = timeout;
        self
    }

    pub(crate) fn with_prefix<E: std::fmt::Display>(mut self, prefix: E) -> Error {
        self.inner.description = format!("{}{}", prefix, self.inner.description);
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.inner.description)?;
        match self.inner.kind {
            Kind::Text(ref text) => {
                write!(f, "{}", text)?;
            }
            Kind::Status(ref code) => {
                let prefix = if code.is_client_error() {
                    "HTTP status client error"
                } else {
                    debug_assert!(code.is_server_error());
                    "HTTP status server error"
                };
                write!(f, "{} ({})", prefix, code)?;
            }
        };

        if let Some(ref t) = self.inner.timeout {
            write!(f, " (timeout {})", t)?;
        }

        if let Some(ref e) = self.inner.source {
            write!(f, ": {}", e)?;
        }

        Ok(())
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut builder = f.debug_struct("reqwest::Error");

        builder.field("kind", &self.inner.kind);

        if let Some(ref source) = self.inner.source {
            builder.field("source", source);
        }

        builder.finish()
    }
}


impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.inner.source.as_ref().map(|e| &**e as _)
    }
}

#[derive(Debug)]
pub(crate) enum Kind {
    Text(String),
    Status(StatusCode),
}

pub(crate) fn text_error(message: String) -> Error {
    Error::new(Kind::Text(message), None::<Error>)
}

pub(crate) fn text_error_with_inner<E: Into<BoxError>>(message: String, e: E) -> Error {
    Error::new(Kind::Text(message), Some(e))
}

pub(crate) fn status_code(status: StatusCode) -> Error {
    Error::new(Kind::Status(status), None::<Error>)
}

pub(crate) fn limited(status: StatusCode, timeout: Option<i64>) -> Error {
    Error::new(Kind::Status(status), None::<Error>).with_timeout(timeout)
}
