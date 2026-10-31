use async_trait::async_trait;

use super::{AccessToken, TokenCredential, oauth::OAuthClient};
use crate::{Result, Secret};

/// Username and password authentication (OAuth resource-owner password flow).
///
/// Legacy flow: it does not work for accounts that require MFA or federated sign-in.
#[derive(Debug, Clone)]
pub struct PasswordCredential {
    oauth: OAuthClient,
    username: String,
    password: Secret,
}

impl PasswordCredential {
    /// Creates a credential for `tenant` (id or domain), `client_id` (public client app),
    /// `username` and `password`.
    pub fn new(
        tenant: impl Into<String>,
        client_id: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<Secret>,
    ) -> Self {
        Self {
            oauth: OAuthClient::new(tenant, client_id),
            username: username.into(),
            password: password.into(),
        }
    }

    /// Overrides the authority host (default `https://login.microsoftonline.com`).
    pub fn with_authority(mut self, authority: impl Into<String>) -> Self {
        self.oauth.authority = authority.into();
        self
    }
}

#[async_trait]
impl TokenCredential for PasswordCredential {
    async fn get_token(&self, scope: &str) -> Result<AccessToken> {
        let r = self
            .oauth
            .token_request(&[
                ("grant_type", "password"),
                ("username", &self.username),
                ("password", self.password.expose()),
                ("scope", scope),
            ])
            .await?;
        Ok(r.token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{body_string_contains, method, path},
    };

    #[tokio::test]
    async fn gets_token() {
        let s = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/t1/oauth2/v2.0/token"))
            .and(body_string_contains("grant_type=password"))
            .and(body_string_contains("username=u%40x.com"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"access_token":"AT","expires_in":3600})),
            )
            .mount(&s)
            .await;
        let c = PasswordCredential::new("t1", "cid", "u@x.com", "pw").with_authority(s.uri());
        assert_eq!(c.get_token("sc").await.unwrap().secret().expose(), "AT");
    }

    #[tokio::test]
    async fn surfaces_service_error() {
        let s = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(400).set_body_json(
                serde_json::json!({"error":"invalid_grant","error_description":"MFA required"}),
            ))
            .mount(&s)
            .await;
        let c = PasswordCredential::new("t1", "cid", "u", "pw").with_authority(s.uri());
        match c.get_token("sc").await.unwrap_err() {
            Error::Auth { code, message } => {
                assert_eq!(code.as_deref(), Some("invalid_grant"));
                assert_eq!(message, "MFA required");
            }
            e => panic!("{e:?}"),
        }
    }

    #[test]
    fn debug_hides_password() {
        let c = PasswordCredential::new("t", "c", "u", "topsecret");
        assert!(!format!("{c:?}").contains("topsecret"));
    }
}
