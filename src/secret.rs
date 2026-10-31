//! A string wrapper that never reveals its contents in `Debug` output.

use std::fmt;

/// Sensitive text such as a password, token or client secret.
///
/// `Debug` prints `[REDACTED]`; use [`Secret::expose`] to read the value.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    /// Wraps a sensitive value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the underlying value. Avoid logging the result.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl From<&str> for Secret {
    fn from(v: &str) -> Self {
        Self::new(v)
    }
}

impl From<String> for Secret {
    fn from(v: String) -> Self {
        Self::new(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_is_redacted() {
        let s = Secret::new("hunter2");
        assert!(!format!("{s:?}").contains("hunter2"));
        assert_eq!(s.expose(), "hunter2");
    }
}
