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
    }
}
