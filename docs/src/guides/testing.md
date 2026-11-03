# Testing your code

You do not need a Microsoft 365 tenant to test code that uses oxipoint. Point the client at a local
mock HTTP server with a fake token. This crate's own tests do exactly that with
[wiremock](https://crates.io/crates/wiremock).

```toml
[dev-dependencies]
wiremock = "0.6"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Graph

`GraphClient::builder(..).base_url(..)` redirects every call:

```rust
use oxipoint::{auth::StaticTokenCredential, graph::GraphClient};
use wiremock::{matchers::{method, path}, Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn reads_me() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "1", "displayName": "Ada"
        })))
        .mount(&server)
        .await;

    let graph = GraphClient::builder(StaticTokenCredential::new("test-token"))
        .base_url(server.uri())
        .build();

    let me = graph.me().get().await.unwrap();
    assert_eq!(me.display_name.as_deref(), Some("Ada"));
}
```

## SharePoint

`SharePointClient::new` takes the site URL, and the mock server's URI works as one:

```rust
use oxipoint::{auth::StaticTokenCredential, sharepoint::SharePointClient};

#[tokio::test]
async fn reads_web() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/_api/web"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "Title": "Team" })))
        .mount(&server)
        .await;

    let sp = SharePointClient::new(&server.uri(), StaticTokenCredential::new("t")).unwrap();
    assert_eq!(sp.web().get().await.unwrap().title.as_deref(), Some("Team"));
}
```

> SharePoint writes first call `POST /_api/contextinfo` for the form digest. Mock it in tests that
> create, update or delete.

## Fast retry tests

When you test your throttling handling, shorten the delay:

```rust
use std::time::Duration;
use oxipoint::http::TransportConfig;

let sp = SharePointClient::with_config(
    &server.uri(),
    StaticTokenCredential::new("t"),
    TransportConfig { max_retries: 2, default_retry_delay: Duration::from_millis(1), ..Default::default() },
)?;
```

## Make your own code testable

Depend on a small trait of your own instead of the whole client, and implement it with the real
client in production and a fake in tests:

```rust
#[async_trait::async_trait]
trait TaskStore {
    async fn open_titles(&self) -> oxipoint::Result<Vec<String>>;
}

struct SpStore(oxipoint::sharepoint::SharePointClient);

#[async_trait::async_trait]
impl TaskStore for SpStore {
    async fn open_titles(&self) -> oxipoint::Result<Vec<String>> {
        let items = self.0.list_by_title("Tasks").items().filter("Status eq 'Open'").list().await?;
        Ok(items.into_iter().filter_map(|i| i.fields.get("Title")?.as_str().map(str::to_owned)).collect())
    }
}
```

## Integration tests against a real tenant

Use a dedicated **test tenant** and a service account or certificate. Keep the IDs in environment
variables, mark the tests `#[ignore]`, and run them explicitly with `cargo test -- --ignored`. Clean
up what you create (the [demo CLI](../reference/cli.md) shows the pattern).
