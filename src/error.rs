//! Error and result types shared by the whole crate.

/// Convenient alias for `Result<T, Error>`.
pub type Result<T> = std::result::Result<T, Error>;

/// All failures the SDK can report.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The identity platform refused or failed to issue a token.
    #[error("authentication failed{}: {message}", code.as_deref().map(|c| format!(" ({c})")).unwrap_or_default())]
    Auth {
        /// Service error code such as `invalid_grant`, if any.
        code: Option<String>,
        /// Human readable description.
        message: String,
    },
    /// Invalid local configuration (bad certificate, malformed URL, ...).
    #[error("configuration error: {0}")]
    Config(String),
    /// The HTTP request could not be completed.
    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),
    /// A response body could not be (de)serialized.
    #[error("serialization error: {0}")]
    Deserialize(#[from] serde_json::Error),
    /// The service answered with a non-success status.
    #[error("HTTP {status}{}: {message}", code.as_deref().map(|c| format!(" ({c})")).unwrap_or_default())]
    Http {
        /// HTTP status code.
        status: u16,
        /// Service specific error code, if present.
        code: Option<String>,
        /// Service error message, or the raw body when unparsable.
        message: String,
        /// Request id reported by the service, if present.
        request_id: Option<String>,
    },
    /// A local I/O failure (for example the loopback listener).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl Error {
    pub(crate) fn auth(code: Option<&str>, message: impl Into<String>) -> Self {
        Error::Auth {
            code: code.map(str::to_owned),
            message: message.into(),
        }
    }

    /// Returns the HTTP status for [`Error::Http`] errors.
    pub fn status(&self) -> Option<u16> {
        match self {
            Error::Http { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// Returns the service error code for [`Error::Http`] and [`Error::Auth`] errors.
    pub fn code(&self) -> Option<&str> {
        match self {
            Error::Http { code, .. } | Error::Auth { code, .. } => code.as_deref(),
            _ => None,
        }
    }

    /// Returns the request id reported by the service, to quote when contacting support.
    pub fn request_id(&self) -> Option<&str> {
        match self {
            Error::Http { request_id, .. } => request_id.as_deref(),
            _ => None,
        }
    }

    /// Whether the service answered `404 Not Found`.
    pub fn is_not_found(&self) -> bool {
        self.status() == Some(404)
    }

    /// Whether the service answered `401 Unauthorized`, or token acquisition failed.
    pub fn is_unauthorized(&self) -> bool {
        matches!(self, Error::Auth { .. }) || self.status() == Some(401)
    }

    /// Whether the service answered `403 Forbidden`.
    pub fn is_forbidden(&self) -> bool {
        self.status() == Some(403)
    }

    /// Whether the service answered `409 Conflict` or `412 Precondition Failed`.
    pub fn is_conflict(&self) -> bool {
        matches!(self.status(), Some(409 | 412))
    }

    /// Whether the request was throttled (`429`), or the service was unavailable (`503`) after
    /// the SDK ran out of retries.
    pub fn is_throttled(&self) -> bool {
        matches!(self.status(), Some(429 | 503))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_includes_code() {
        let e = Error::auth(Some("invalid_grant"), "bad");
        assert_eq!(e.to_string(), "authentication failed (invalid_grant): bad");
        let e = Error::Http {
            status: 404,
            code: None,
            message: "nope".into(),
            request_id: None,
        };
        assert_eq!(e.to_string(), "HTTP 404: nope");
        assert_eq!(e.status(), Some(404));
        assert!(e.is_not_found() && !e.is_throttled());
    }

    #[test]
    fn classifiers_and_accessors() {
        let http = |status| Error::Http {
            status,
            code: Some("c".into()),
            message: String::new(),
            request_id: Some("r".into()),
        };
        assert!(http(429).is_throttled() && http(503).is_throttled());
        assert!(http(401).is_unauthorized() && http(403).is_forbidden());
        assert!(http(409).is_conflict() && http(412).is_conflict());
        assert_eq!(http(500).code(), Some("c"));
        assert_eq!(http(500).request_id(), Some("r"));
        assert!(Error::auth(None, "x").is_unauthorized());
        assert_eq!(Error::Config("x".into()).code(), None);
    }
}
