use std::{
    fmt,
    sync::Arc,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use serde::Deserialize;
use tokio::sync::Mutex;

use super::{
    AccessToken, TokenCredential,
    oauth::{OAuthClient, with_offline},
};
use crate::{Error, Result, Secret};

/// Instructions to show the user during the device code flow.
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCodeInfo {
    /// Code the user must enter.
    pub user_code: String,
    /// Page where the code is entered.
    pub verification_uri: String,
    /// Ready-made message for the user.
    #[serde(default)]
    pub message: String,
    /// Seconds before the code expires.
    pub expires_in: u64,
    /// Polling interval in seconds.
    #[serde(default = "default_interval")]
    pub interval: u64,
    #[serde(skip)]
    device_code: String,
}

fn default_interval() -> u64 {
    5
}

#[derive(Deserialize)]
struct RawDeviceCode {
    device_code: String,
    #[serde(flatten)]
    info: DeviceCodeInfo,
}

type Prompt = Arc<dyn Fn(&DeviceCodeInfo) + Send + Sync>;

/// Device code authentication: the user signs in on another device.
///
/// The callback receives the verification URL and code to display.
pub struct DeviceCodeCredential {
    oauth: OAuthClient,
    prompt: Prompt,
    refresh: Mutex<Option<Secret>>,
    slow_down_step: Duration,
}

impl fmt::Debug for DeviceCodeCredential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeviceCodeCredential")
            .field("client_id", &self.oauth.client_id)
            .finish_non_exhaustive()
    }
}

impl DeviceCodeCredential {
    /// Creates a credential; `prompt` is called once per sign-in with the instructions.
    pub fn new(
        tenant: impl Into<String>,
        client_id: impl Into<String>,
        prompt: impl Fn(&DeviceCodeInfo) + Send + Sync + 'static,
    ) -> Self {
        Self {
            oauth: OAuthClient::new(tenant, client_id),
            prompt: Arc::new(prompt),
            refresh: Mutex::new(None),
            slow_down_step: Duration::from_secs(5),
        }
    }

    /// Overrides the authority host.
    pub fn with_authority(mut self, authority: impl Into<String>) -> Self {
        self.oauth.authority = authority.into();
        self
    }

    #[cfg(test)]
    fn with_slow_down_step(mut self, d: Duration) -> Self {
        self.slow_down_step = d;
        self
    }
}

#[async_trait]
impl TokenCredential for DeviceCodeCredential {
    async fn get_token(&self, scope: &str) -> Result<AccessToken> {
        let mut refresh = self.refresh.lock().await;
        if let Some(rt) = refresh.clone()
            && let Ok(r) = self.oauth.refresh(&rt, scope).await
        {
            *refresh = r.refresh_token.or(Some(rt));
            return Ok(r.token);
        }

        let raw: RawDeviceCode = self
            .oauth
            .post_form(
                &self.oauth.endpoint("devicecode"),
                &[
                    ("client_id", &self.oauth.client_id),
                    ("scope", &with_offline(scope)),
                ],
            )
            .await?;
        let mut info = raw.info;
        info.device_code = raw.device_code;
        (self.prompt)(&info);

        let deadline = Instant::now() + Duration::from_secs(info.expires_in);
        let mut interval = Duration::from_secs(info.interval);
        loop {
            tokio::time::sleep(interval).await;
            if Instant::now() >= deadline {
                return Err(Error::auth(
                    Some("expired_token"),
                    "the device code expired",
                ));
            }
            let res = self
                .oauth
                .token_request(&[
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                    ("device_code", &info.device_code),
                ])
                .await;
            match res {
                Ok(r) => {
                    *refresh = r.refresh_token;
                    return Ok(r.token);
                }
                Err(Error::Auth { code: Some(c), .. }) if c == "authorization_pending" => {}
                Err(Error::Auth { code: Some(c), .. }) if c == "slow_down" => {
                    interval += self.slow_down_step
                }
                Err(e) => return Err(e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Mutex as StdMutex;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{body_string_contains, method, path},
    };

    async fn start(responses: Vec<ResponseTemplate>) -> MockServer {
        let s = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/t/oauth2/v2.0/devicecode"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "device_code":"DC","user_code":"UC","verification_uri":"https://microsoft.com/devicelogin",
                "expires_in":900,"interval":0,"message":"go"
            })))
            .mount(&s)
            .await;
        for r in responses {
            Mock::given(method("POST"))
                .and(path("/t/oauth2/v2.0/token"))
                .and(body_string_contains("device_code=DC"))
                .respond_with(r)
                .up_to_n_times(1)
                .mount(&s)
                .await;
        }
        s
    }

    fn err(code: &str) -> ResponseTemplate {
        ResponseTemplate::new(400).set_body_json(json!({"error":code,"error_description":"d"}))
    }

    #[tokio::test]
    async fn prompts_polls_and_succeeds() {
        let s = start(vec![
            err("authorization_pending"),
            err("slow_down"),
            ResponseTemplate::new(200)
                .set_body_json(json!({"access_token":"AT","expires_in":3600,"refresh_token":"RT"})),
        ])
        .await;
        let shown = Arc::new(StdMutex::new(Vec::new()));
        let sh = shown.clone();
        let c = DeviceCodeCredential::new("t", "cid", move |i| {
            sh.lock().unwrap().push(i.user_code.clone())
        })
        .with_authority(s.uri())
        .with_slow_down_step(Duration::ZERO);
        let t = c.get_token("sc").await.unwrap();
        assert_eq!(t.secret().expose(), "AT");
        assert_eq!(*shown.lock().unwrap(), vec!["UC".to_string()]);
        assert!(c.refresh.lock().await.is_some());
    }

    #[tokio::test]
    async fn declined_is_error() {
        let s = start(vec![err("authorization_declined")]).await;
        let c = DeviceCodeCredential::new("t", "cid", |_| {}).with_authority(s.uri());
        let e = c.get_token("sc").await.unwrap_err();
        assert!(matches!(e, Error::Auth { code: Some(c), .. } if c == "authorization_declined"));
    }

    #[tokio::test]
    async fn expired_code_is_error() {
        let s = MockServer::start().await;
        Mock::given(path("/t/oauth2/v2.0/devicecode"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "device_code":"DC","user_code":"UC","verification_uri":"u","expires_in":0,"interval":0
            })))
            .mount(&s)
            .await;
        let c = DeviceCodeCredential::new("t", "cid", |_| {}).with_authority(s.uri());
        tokio::time::sleep(Duration::from_millis(5)).await;
        let e = c.get_token("sc").await.unwrap_err();
        assert!(matches!(e, Error::Auth { code: Some(c), .. } if c == "expired_token"));
    }
}
