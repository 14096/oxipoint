//! Authentication: obtaining and caching Microsoft identity platform tokens.
//!
//! Every flow implements [`TokenCredential`], so clients accept any of them (or your own).
//!
//! | Flow | Type | Feature |
//! |------|------|---------|
//! | Username + password (ROPC, no MFA) | [`PasswordCredential`] | `auth-password` |
//! | Client certificate (app-only) | [`CertificateCredential`] | `auth-certificate` |
//! | Interactive browser (code + PKCE) | [`BrowserCredential`] | `auth-browser` |
//! | Device code | [`DeviceCodeCredential`] | `auth-device-code` |

use std::{
    collections::HashMap,
    fmt::Debug,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::{Result, Secret};

pub(crate) mod oauth;

#[cfg(feature = "auth-browser")]
mod browser;
#[cfg(feature = "auth-certificate")]
mod certificate;
#[cfg(feature = "auth-device-code")]
mod device_code;
#[cfg(feature = "auth-password")]
mod password;

#[cfg(feature = "auth-browser")]
pub use browser::BrowserCredential;
#[cfg(feature = "auth-certificate")]
pub use certificate::CertificateCredential;
#[cfg(feature = "auth-device-code")]
pub use device_code::{DeviceCodeCredential, DeviceCodeInfo};
#[cfg(feature = "auth-password")]
pub use password::PasswordCredential;

/// Tokens are renewed when they expire within this window.
const EXPIRY_SKEW: Duration = Duration::from_secs(120);

/// A bearer access token with its expiry. `Debug` never prints the token.
#[derive(Debug, Clone)]
pub struct AccessToken {
    secret: Secret,
    expires_at: Instant,
}

impl AccessToken {
    /// Creates a token valid for `lifetime` from now.
    pub fn new(token: impl Into<Secret>, lifetime: Duration) -> Self {
        Self {
            secret: token.into(),
            expires_at: Instant::now() + lifetime,
        }
    }

    /// The token value.
    pub fn secret(&self) -> &Secret {
        &self.secret
    }

    /// Whether the token is expired or about to expire.
    pub fn is_expiring(&self) -> bool {
        Instant::now() + EXPIRY_SKEW >= self.expires_at
    }
}

/// Source of access tokens. Implement this to add a custom authentication flow.
#[async_trait]
pub trait TokenCredential: Send + Sync + Debug {
    /// Returns a valid token for `scope` (for example `https://graph.microsoft.com/.default`).
    async fn get_token(&self, scope: &str) -> Result<AccessToken>;
}

/// Wraps a credential and caches tokens per scope until shortly before they expire.
#[derive(Debug)]
pub struct CachedCredential<C> {
    inner: C,
    cache: Mutex<HashMap<String, AccessToken>>,
}

impl<C: TokenCredential> CachedCredential<C> {
    /// Adds caching to `inner`.
    pub fn new(inner: C) -> Self {
        Self {
            inner,
            cache: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl<C: TokenCredential> TokenCredential for CachedCredential<C> {
    async fn get_token(&self, scope: &str) -> Result<AccessToken> {
        // The lock is held during renewal so concurrent callers share one sign-in.
        let mut cache = self.cache.lock().await;
        if let Some(t) = cache.get(scope).filter(|t| !t.is_expiring()) {
            return Ok(t.clone());
        }
        let token = self.inner.get_token(scope).await?;
        cache.insert(scope.to_owned(), token.clone());
        Ok(token)
    }
}

/// A credential that always returns a pre-obtained token (useful for tests or external auth).
#[derive(Debug, Clone)]
pub struct StaticTokenCredential {
    token: Secret,
}

impl StaticTokenCredential {
    /// Uses `token` for every request.
    pub fn new(token: impl Into<Secret>) -> Self {
        Self {
            token: token.into(),
        }
    }
}

#[async_trait]
impl TokenCredential for StaticTokenCredential {
    async fn get_token(&self, _scope: &str) -> Result<AccessToken> {
        Ok(AccessToken::new(
            self.token.clone(),
            Duration::from_secs(3600),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug, Default)]
    struct Counting {
        calls: AtomicUsize,
        lifetime_secs: u64,
    }

    #[async_trait]
    impl TokenCredential for Counting {
        async fn get_token(&self, _s: &str) -> Result<AccessToken> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(AccessToken::new(
                "t",
                Duration::from_secs(self.lifetime_secs),
            ))
        }
    }

    #[tokio::test]
    async fn caches_per_scope() {
        let c = CachedCredential::new(Counting {
            lifetime_secs: 3600,
            ..Default::default()
        });
        c.get_token("a").await.unwrap();
        c.get_token("a").await.unwrap();
        c.get_token("b").await.unwrap();
        assert_eq!(c.inner.calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn renews_expiring_tokens() {
        let c = CachedCredential::new(Counting {
            lifetime_secs: 10,
            ..Default::default()
        });
        c.get_token("a").await.unwrap();
        c.get_token("a").await.unwrap();
        assert_eq!(c.inner.calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn token_debug_is_redacted() {
        let t = AccessToken::new("supersecret", Duration::from_secs(1));
        assert!(!format!("{t:?}").contains("supersecret"));
    }
}
