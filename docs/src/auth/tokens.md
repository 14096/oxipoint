# Tokens, security and custom credentials

## Caching

Clients wrap the credential in a cache:

- Tokens are kept **per scope** and renewed about **two minutes** before they expire.
- Concurrent requests **share one sign-in**: ten tasks starting together cause one token request.
- Browser and device code credentials keep the **refresh token** and use it to get tokens for other
  scopes (Graph, then SharePoint) without prompting again.

## One credential instance per client

Credentials other than `PasswordCredential` are not `Clone`. A second client built from a second
instance signs in again. If you need both clients and want one prompt, build one credential and
wrap it behind your own shared type implementing `TokenCredential` (below), or use two instances
and accept two prompts.

## Secrets are redacted

Passwords, access tokens and refresh tokens are wrapped in `Secret` and print as `[REDACTED]` in
`Debug` output. Errors never contain them. Call `secret.expose()` only when you truly need the raw
string.

```rust
use oxipoint::Secret;

let s = Secret::new("hunter2");
println!("{s:?}");          // [REDACTED]
let raw: &str = s.expose();
```

## Static tokens

`auth::StaticTokenCredential::new("<token>")` covers the simple case of a token you already
acquired elsewhere (for example from an Azure Functions binding or a test fixture). It does not
refresh.

```rust
use oxipoint::auth::StaticTokenCredential;

let graph = GraphClient::new(StaticTokenCredential::new(std::env::var("GRAPH_TOKEN").unwrap()));
```

## Custom credentials

Implement the trait to plug in anything: managed identity, a token from another library, a
company token broker.

```rust
use async_trait::async_trait;
use oxipoint::auth::{AccessToken, TokenCredential};
use std::time::Duration;

#[derive(Debug)]
struct MyCredential;

#[async_trait]
impl TokenCredential for MyCredential {
    async fn get_token(&self, scope: &str) -> oxipoint::Result<AccessToken> {
        // `scope` is e.g. "https://graph.microsoft.com/.default"
        Ok(AccessToken::new("<token for scope>", Duration::from_secs(3600)))
    }
}
```

Return the real lifetime in `AccessToken::new`: the cache uses it to decide when to ask again.

### Sharing one credential between clients

```rust
use std::sync::Arc;
use oxipoint::auth::TokenCredential as _;

#[derive(Debug, Clone)]
struct Shared(Arc<oxipoint::auth::DeviceCodeCredential>);

#[async_trait::async_trait]
impl oxipoint::auth::TokenCredential for Shared {
    async fn get_token(&self, scope: &str) -> oxipoint::Result<oxipoint::auth::AccessToken> {
        self.0.get_token(scope).await
    }
}

let shared = Shared(Arc::new(cred));
let graph = GraphClient::new(shared.clone());
let sp = SharePointClient::new(site_url, shared)?;
```

## Security checklist

- Keep client IDs, tenant IDs and secrets in environment variables or a secret store.
- Prefer certificate or delegated flows over passwords.
- Grant the least permission that works; prefer `Sites.Selected`.
- Never log raw tokens. `Secret` helps, but only if you do not call `expose()` into a log line.
