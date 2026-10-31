use std::{fmt, sync::Arc, time::Duration};

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::Mutex,
};
use url::Url;

use super::{
    AccessToken, TokenCredential,
    oauth::{OAuthClient, random_urlsafe, with_offline},
};
use crate::{Error, Result, Secret};

type Opener = Arc<dyn Fn(&str) -> std::io::Result<()> + Send + Sync>;

/// Interactive sign-in: authorization code flow with PKCE.
///
/// Opens the system browser, then waits for the redirect on `http://localhost:<port>`.
/// The app registration must allow that redirect URI for a public client.
pub struct BrowserCredential {
    oauth: OAuthClient,
    port: u16,
    timeout: Duration,
    opener: Opener,
    refresh: Mutex<Option<Secret>>,
}

impl fmt::Debug for BrowserCredential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BrowserCredential")
            .field("client_id", &self.oauth.client_id)
            .finish_non_exhaustive()
    }
}

impl BrowserCredential {
    /// Creates a credential using a random free loopback port and a 5 minute timeout.
    pub fn new(tenant: impl Into<String>, client_id: impl Into<String>) -> Self {
        Self {
            oauth: OAuthClient::new(tenant, client_id),
            port: 0,
            timeout: Duration::from_secs(300),
            opener: Arc::new(|url| open::that(url)),
            refresh: Mutex::new(None),
        }
    }

    /// Uses a fixed loopback port (needed when the app registration pins the redirect URI).
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Maximum time to wait for the user to finish signing in.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Replaces how the sign-in URL is presented (default: open the system browser).
    pub fn with_url_opener(
        mut self,
        f: impl Fn(&str) -> std::io::Result<()> + Send + Sync + 'static,
    ) -> Self {
        self.opener = Arc::new(f);
        self
    }

    /// Overrides the authority host.
    pub fn with_authority(mut self, authority: impl Into<String>) -> Self {
        self.oauth.authority = authority.into();
        self
    }

    fn authorize_url(
        &self,
        scope: &str,
        redirect: &str,
        state: &str,
        challenge: &str,
    ) -> Result<String> {
        let mut url = Url::parse(&self.oauth.endpoint("authorize"))
            .map_err(|e| Error::Config(e.to_string()))?;
        url.query_pairs_mut()
            .append_pair("client_id", &self.oauth.client_id)
            .append_pair("response_type", "code")
            .append_pair("redirect_uri", redirect)
            .append_pair("response_mode", "query")
            .append_pair("scope", &with_offline(scope))
            .append_pair("state", state)
            .append_pair("code_challenge", challenge)
            .append_pair("code_challenge_method", "S256");
        Ok(url.into())
    }
}

/// Waits for the redirect carrying `code`/`error` and returns its query parameters.
async fn receive_redirect(listener: &TcpListener) -> Result<Vec<(String, String)>> {
    loop {
        let (stream, _) = listener.accept().await?;
        if let Some(params) = handle_connection(stream).await? {
            return Ok(params);
        }
    }
}

async fn handle_connection(mut stream: TcpStream) -> Result<Option<Vec<(String, String)>>> {
    let mut line = String::new();
    BufReader::new(&mut stream).read_line(&mut line).await?;
    let target = line.split_whitespace().nth(1).unwrap_or("/");
    let params: Vec<(String, String)> = Url::parse(&format!("http://localhost{target}"))
        .map(|u| {
            u.query_pairs()
                .map(|(k, v)| (k.into_owned(), v.into_owned()))
                .collect()
        })
        .unwrap_or_default();
    let relevant = params.iter().any(|(k, _)| k == "code" || k == "error");
    let (status, body) = if relevant {
        ("200 OK", "Sign-in complete. You can close this window.")
    } else {
        ("404 Not Found", "Not found")
    };
    let resp = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(resp.as_bytes()).await?;
    stream.shutdown().await.ok();
    Ok(relevant.then_some(params))
}

fn param<'a>(params: &'a [(String, String)], key: &str) -> Option<&'a str> {
    params
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

#[async_trait]
impl TokenCredential for BrowserCredential {
    async fn get_token(&self, scope: &str) -> Result<AccessToken> {
        let mut refresh = self.refresh.lock().await;
        if let Some(rt) = refresh.clone()
            && let Ok(r) = self.oauth.refresh(&rt, scope).await
        {
            *refresh = r.refresh_token.or(Some(rt));
            return Ok(r.token);
        }

        let listener = TcpListener::bind(("127.0.0.1", self.port)).await?;
        let redirect = format!("http://localhost:{}", listener.local_addr()?.port());
        let verifier = random_urlsafe::<32>();
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        let state = random_urlsafe::<16>();
        let url = self.authorize_url(scope, &redirect, &state, &challenge)?;
        (self.opener)(&url)?;

        let params = tokio::time::timeout(self.timeout, receive_redirect(&listener))
            .await
            .map_err(|_| Error::auth(Some("timeout"), "timed out waiting for browser sign-in"))??;
        if let Some(err) = param(&params, "error") {
            return Err(Error::auth(
                Some(err),
                param(&params, "error_description").unwrap_or_default(),
            ));
        }
        if param(&params, "state") != Some(state.as_str()) {
            return Err(Error::auth(
                Some("state_mismatch"),
                "redirect state does not match the request",
            ));
        }
        let code = param(&params, "code")
            .ok_or_else(|| Error::auth(None, "redirect carried no authorization code"))?;

        let r = self
            .oauth
            .token_request(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("redirect_uri", &redirect),
                ("code_verifier", &verifier),
                ("scope", &with_offline(scope)),
            ])
            .await?;
        *refresh = r.refresh_token;
        Ok(r.token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{body_string_contains, method, path},
    };

    /// Simulates the browser: parses the sign-in URL and calls back with `build(state)`.
    fn fake_browser(
        build: impl Fn(&str) -> String + Send + Sync + 'static,
    ) -> impl Fn(&str) -> std::io::Result<()> {
        move |url| {
            let u = Url::parse(url).unwrap();
            let q: std::collections::HashMap<_, _> = u.query_pairs().into_owned().collect();
            assert_eq!(q["code_challenge_method"], "S256");
            let target = format!(
                "{}{}",
                q["redirect_uri"].replace("localhost", "127.0.0.1"),
                build(&q["state"])
            );
            tokio::spawn(async move {
                reqwest::get(target).await.ok();
            });
            Ok(())
        }
    }

    async fn token_server() -> MockServer {
        let s = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/t/oauth2/v2.0/token"))
            .and(body_string_contains("grant_type=authorization_code"))
            .and(body_string_contains("code=THECODE"))
            .and(body_string_contains("code_verifier="))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"access_token":"AT","expires_in":3600})),
            )
            .mount(&s)
            .await;
        s
    }

    #[tokio::test]
    async fn completes_sign_in() {
        let s = token_server().await;
        let c = BrowserCredential::new("t", "cid")
            .with_authority(s.uri())
            .with_url_opener(fake_browser(|state| {
                format!("/?code=THECODE&state={state}")
            }));
        assert_eq!(c.get_token("sc").await.unwrap().secret().expose(), "AT");
    }

    #[tokio::test]
    async fn rejects_state_mismatch() {
        let s = token_server().await;
        let c = BrowserCredential::new("t", "cid")
            .with_authority(s.uri())
            .with_url_opener(fake_browser(|_| "/?code=THECODE&state=forged".into()));
        let e = c.get_token("sc").await.unwrap_err();
        assert!(matches!(e, Error::Auth { code: Some(c), .. } if c == "state_mismatch"));
        assert!(s.received_requests().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn reports_denied_sign_in() {
        let s = token_server().await;
        let c = BrowserCredential::new("t", "cid")
            .with_authority(s.uri())
            .with_url_opener(fake_browser(|state| {
                format!("/?error=access_denied&error_description=no&state={state}")
            }));
        let e = c.get_token("sc").await.unwrap_err();
        assert!(matches!(e, Error::Auth { code: Some(c), .. } if c == "access_denied"));
    }

    #[tokio::test]
    async fn times_out() {
        let c = BrowserCredential::new("t", "cid")
            .with_timeout(Duration::from_millis(50))
            .with_url_opener(|_| Ok(()));
        let e = c.get_token("sc").await.unwrap_err();
        assert!(matches!(e, Error::Auth { code: Some(c), .. } if c == "timeout"));
    }
}
