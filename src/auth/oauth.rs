//! Minimal OAuth 2.0 client for the Microsoft identity platform v2.0 endpoints.

// Some helpers are only used by individual auth features.
#![allow(dead_code)]

use std::time::Duration;

use serde::Deserialize;

use super::AccessToken;
use crate::{Error, Result, Secret};

pub(crate) const DEFAULT_AUTHORITY: &str = "https://login.microsoftonline.com";

#[derive(Debug, Clone)]
pub(crate) struct OAuthClient {
    http: reqwest::Client,
    pub(crate) authority: String,
    pub(crate) tenant: String,
    pub(crate) client_id: String,
}

#[derive(Debug, Deserialize)]
struct RawTokenResponse {
    access_token: String,
    #[serde(default)]
    expires_in: Option<u64>,
    #[serde(default)]
    refresh_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawError {
    error: Option<String>,
    error_description: Option<String>,
}

pub(crate) struct TokenResponse {
    pub(crate) token: AccessToken,
    pub(crate) refresh_token: Option<Secret>,
}

impl OAuthClient {
    pub(crate) fn new(tenant: impl Into<String>, client_id: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            authority: DEFAULT_AUTHORITY.to_owned(),
            tenant: tenant.into(),
            client_id: client_id.into(),
        }
    }

    pub(crate) fn endpoint(&self, name: &str) -> String {
        format!(
            "{}/{}/oauth2/v2.0/{}",
            self.authority.trim_end_matches('/'),
            self.tenant,
            name
        )
    }

    pub(crate) fn token_url(&self) -> String {
        self.endpoint("token")
    }

    /// POSTs a form to `url` and returns the parsed JSON body or an [`Error::Auth`].
    pub(crate) async fn post_form<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        params: &[(&str, &str)],
    ) -> Result<T> {
        let resp = self.http.post(url).form(params).send().await?;
        let status = resp.status();
        let body = resp.bytes().await?;
        if status.is_success() {
            return Ok(serde_json::from_slice(&body)?);
        }
        match serde_json::from_slice::<RawError>(&body) {
            Ok(e) if e.error.is_some() => Err(Error::auth(
                e.error.as_deref(),
                e.error_description.unwrap_or_default(),
            )),
            _ => Err(Error::auth(
                None,
                format!("HTTP {status}: {}", String::from_utf8_lossy(&body)),
            )),
        }
    }

    /// Requests a token with the given grant parameters (`client_id` is added).
    pub(crate) async fn token_request(&self, params: &[(&str, &str)]) -> Result<TokenResponse> {
        let mut all = vec![("client_id", self.client_id.as_str())];
        all.extend_from_slice(params);
        let raw: RawTokenResponse = self.post_form(&self.token_url(), &all).await?;
        Ok(TokenResponse {
            token: AccessToken::new(
                raw.access_token,
                Duration::from_secs(raw.expires_in.unwrap_or(3600)),
            ),
            refresh_token: raw.refresh_token.map(Secret::new),
        })
    }

    pub(crate) async fn refresh(
        &self,
        refresh_token: &Secret,
        scope: &str,
    ) -> Result<TokenResponse> {
        self.token_request(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.expose()),
            ("scope", &with_offline(scope)),
        ])
        .await
    }
}

/// Appends `offline_access` so a refresh token is issued.
pub(crate) fn with_offline(scope: &str) -> String {
    format!("{scope} offline_access")
}

pub(crate) fn random_urlsafe<const N: usize>() -> String {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    URL_SAFE_NO_PAD.encode(rand::random::<[u8; N]>())
}
