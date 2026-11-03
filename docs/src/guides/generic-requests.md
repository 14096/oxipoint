# Generic requests

Typed wrappers cover the common ground. For everything else, both clients expose
`request(method, path)`. It uses the **same authentication, throttling retries and error
mapping** as the typed calls.

```rust
use reqwest::Method;
```

## The builder

`request` returns a builder with:

| Method               | Purpose                                                       |
| -------------------- | ------------------------------------------------------------- |
| `query(key, value)`  | Add one query-string parameter                                |
| `options(&opts)`     | Add a `QueryOptions` (`select`, `filter`, ...)                |
| `header(name, value)`| Add a header                                                  |
| `json(&body)`        | JSON body                                                     |
| `body(bytes)`        | Raw body                                                      |
| `send()`             | Send, return a `Response` (`json()`, `text()`)                |
| `send_json::<T>()`   | Send and deserialize                                          |
| `send_stream()`      | Send and read the body in chunks                              |
| `copy_to(&mut w)`    | Stream the body into an `AsyncWrite`                          |
| `pager::<T>()`       | Treat the endpoint as a paged collection                      |

The path is **relative to the client's base URL** (`/me/messages`, `/web/currentuser`) or an
absolute URL on the **same host**. A URL on another host fails with `Error::Config`.

## Read something

```rust
let recent: serde_json::Value = graph
    .request(Method::GET, "/me/messages")
    .query("$top", "5")
    .send_json()
    .await?;
```

Into your own type:

```rust
#[derive(serde::Deserialize)]
struct Message { subject: Option<String> }

#[derive(serde::Deserialize)]
struct Page { value: Vec<Message> }

let page: Page = graph.request(Method::GET, "/me/messages").query("$top", "5").send_json().await?;
```

## Write something

```rust
graph.request(Method::POST, "/me/sendMail")
    .json(&serde_json::json!({
        "message": {
            "subject": "Hi",
            "body": { "contentType": "Text", "content": "Hello" },
            "toRecipients": [{ "emailAddress": { "address": "a@contoso.com" } }]
        }
    }))
    .send()
    .await?;
```

## Any paged collection

```rust
#[derive(serde::Deserialize)]
struct Group { id: String }

let groups: Vec<Group> = graph.request(Method::GET, "/groups").pager()?.collect_all().await?;
```

## Streaming a large download

```rust
let mut out = tokio::fs::File::create("export.csv").await?;
let bytes = graph.request(Method::GET, "/some/large/export").copy_to(&mut out).await?;
```

## SharePoint

```rust
let user: serde_json::Value = sp.request(Method::GET, "/web/currentuser").send_json().await?;
```

For **writes** to SharePoint through the generic builder, send an `X-RequestDigest` header yourself
(`POST /contextinfo` returns the value). The typed write methods do this automatically.

```rust
use serde_json::Value;

let ctx: Value = sp.request(Method::POST, "/contextinfo").send_json().await?;
let digest = ctx["FormDigestValue"].as_str().unwrap_or_default();

sp.request(Method::POST, "/web/lists/getbytitle('Tasks')/items")
    .header("X-RequestDigest", digest)
    .json(&serde_json::json!({ "Title": "via generic" }))
    .send()
    .await?;
```
