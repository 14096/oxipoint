use std::{
    fmt,
    time::{SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{
    AccessToken, TokenCredential,
    oauth::{OAuthClient, random_urlsafe},
};
use crate::{Error, Result};

#[derive(Serialize)]
struct Claims<'a> {
    aud: &'a str,
    iss: &'a str,
    sub: &'a str,
    jti: String,
    nbf: u64,
    exp: u64,
}

/// App-only authentication with a client certificate (client credentials grant using a
/// signed JWT assertion). The tenant must have the certificate registered on the app.
pub struct CertificateCredential {
    oauth: OAuthClient,
    key: EncodingKey,
    thumbprint_s256: String,
}

impl fmt::Debug for CertificateCredential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CertificateCredential")
            .field("client_id", &self.oauth.client_id)
            .field("thumbprint_s256", &self.thumbprint_s256)
            .finish_non_exhaustive()
    }
}

impl CertificateCredential {
    /// Builds a credential from a PEM certificate and its PEM RSA private key
    /// (PKCS#8 or PKCS#1). Parsing failures are reported before any network call.
    pub fn from_pem(
        tenant: impl Into<String>,
        client_id: impl Into<String>,
        cert_pem: &str,
        key_pem: &str,
    ) -> Result<Self> {
        let der = pem_certificate_der(cert_pem)?;
        let key = EncodingKey::from_rsa_pem(key_pem.as_bytes())
            .map_err(|e| Error::Config(format!("invalid RSA private key: {e}")))?;
        Ok(Self::new(tenant, client_id, key, &der))
    }

    /// Builds a credential from a PKCS#12 (PFX) archive and its password.
    pub fn from_pfx(
        tenant: impl Into<String>,
        client_id: impl Into<String>,
        pfx: &[u8],
        password: &str,
    ) -> Result<Self> {
        use p12_keystore::{KeyStore, Pkcs12ImportPolicy};
        let store = KeyStore::from_pkcs12(pfx, password, Pkcs12ImportPolicy::Relaxed)
            .map_err(|e| Error::Config(format!("invalid PFX: {e}")))?;
        let (_, chain) = store
            .private_key_chain()
            .ok_or_else(|| Error::Config("PFX contains no private key".into()))?;
        let cert = chain
            .certs()
            .first()
            .ok_or_else(|| Error::Config("PFX contains no certificate".into()))?;
        // The archive holds PKCS#8 DER; re-wrap as PEM so the signer accepts it.
        let pem = format!(
            "-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----\n",
            STANDARD.encode(chain.key().as_der())
        );
        let key = EncodingKey::from_rsa_pem(pem.as_bytes())
            .map_err(|e| Error::Config(format!("PFX key is not a usable RSA key: {e}")))?;
        Ok(Self::new(tenant, client_id, key, cert.as_der()))
    }

    fn new(
        tenant: impl Into<String>,
        client_id: impl Into<String>,
        key: EncodingKey,
        cert_der: &[u8],
    ) -> Self {
        Self {
            oauth: OAuthClient::new(tenant, client_id),
            key,
            thumbprint_s256: URL_SAFE_NO_PAD.encode(Sha256::digest(cert_der)),
        }
    }

    /// Overrides the authority host.
    pub fn with_authority(mut self, authority: impl Into<String>) -> Self {
        self.oauth.authority = authority.into();
        self
    }

    fn assertion(&self) -> Result<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let url = self.oauth.token_url();
        let claims = Claims {
            aud: &url,
            iss: &self.oauth.client_id,
            sub: &self.oauth.client_id,
            jti: random_urlsafe::<16>(),
            nbf: now,
            exp: now + 600,
        };
        let mut header = Header::new(Algorithm::RS256);
        header.x5t_s256 = Some(self.thumbprint_s256.clone());
        jsonwebtoken::encode(&header, &claims, &self.key)
            .map_err(|e| Error::Config(format!("cannot sign assertion: {e}")))
    }
}

fn pem_certificate_der(pem: &str) -> Result<Vec<u8>> {
    const BEGIN: &str = "-----BEGIN CERTIFICATE-----";
    const END: &str = "-----END CERTIFICATE-----";
    let start = pem
        .find(BEGIN)
        .ok_or_else(|| Error::Config("no PEM certificate found".into()))?
        + BEGIN.len();
    let end = pem[start..]
        .find(END)
        .ok_or_else(|| Error::Config("unterminated PEM certificate".into()))?
        + start;
    let b64: String = pem[start..end]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    STANDARD
        .decode(b64)
        .map_err(|e| Error::Config(format!("invalid certificate encoding: {e}")))
}

#[async_trait]
impl TokenCredential for CertificateCredential {
    async fn get_token(&self, scope: &str) -> Result<AccessToken> {
        let assertion = self.assertion()?;
        let r = self
            .oauth
            .token_request(&[
                ("grant_type", "client_credentials"),
                ("scope", scope),
                (
                    "client_assertion_type",
                    "urn:ietf:params:oauth:client-assertion-type:jwt-bearer",
                ),
                ("client_assertion", &assertion),
            ])
            .await?;
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

    const CERT: &str = include_str!("testdata/cert.pem");
    const KEY: &str = include_str!("testdata/key.pem");
    const PFX: &[u8] = include_bytes!("testdata/cert.pfx");

    fn decode_part(jwt: &str, idx: usize) -> serde_json::Value {
        serde_json::from_slice(
            &URL_SAFE_NO_PAD
                .decode(jwt.split('.').nth(idx).unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn assertion_has_expected_header_and_claims() {
        let c = CertificateCredential::from_pem("t1", "cid", CERT, KEY).unwrap();
        let jwt = c.assertion().unwrap();
        let h = decode_part(&jwt, 0);
        assert_eq!(h["alg"], "RS256");
        assert_eq!(h["x5t#S256"], c.thumbprint_s256.as_str());
        let claims = decode_part(&jwt, 1);
        assert_eq!(claims["iss"], "cid");
        assert_eq!(claims["sub"], "cid");
        assert_eq!(
            claims["aud"],
            "https://login.microsoftonline.com/t1/oauth2/v2.0/token"
        );
    }

    #[test]
    fn pfx_and_pem_agree_on_thumbprint() {
        let a = CertificateCredential::from_pem("t", "c", CERT, KEY).unwrap();
        let b = CertificateCredential::from_pfx("t", "c", PFX, "pw").unwrap();
        assert_eq!(a.thumbprint_s256, b.thumbprint_s256);
        assert!(b.assertion().is_ok());
    }

    #[test]
    fn invalid_material_is_config_error() {
        assert!(matches!(
            CertificateCredential::from_pem("t", "c", "junk", KEY),
            Err(Error::Config(_))
        ));
        assert!(matches!(
            CertificateCredential::from_pem("t", "c", CERT, "junk"),
            Err(Error::Config(_))
        ));
        assert!(matches!(
            CertificateCredential::from_pfx("t", "c", b"junk", "pw"),
            Err(Error::Config(_))
        ));
        assert!(matches!(
            CertificateCredential::from_pfx("t", "c", PFX, "wrong"),
            Err(Error::Config(_))
        ));
    }

    #[tokio::test]
    async fn exchanges_assertion_for_token() {
        let s = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/t1/oauth2/v2.0/token"))
            .and(body_string_contains("grant_type=client_credentials"))
            .and(body_string_contains("client_assertion="))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"access_token":"AT","expires_in":3600})),
            )
            .mount(&s)
            .await;
        let c = CertificateCredential::from_pem("t1", "cid", CERT, KEY)
            .unwrap()
            .with_authority(s.uri());
        assert_eq!(c.get_token("sc").await.unwrap().secret().expose(), "AT");
    }
}
