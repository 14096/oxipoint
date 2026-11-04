# Errors, retries and throttling

## The error type

All fallible calls return `oxipoint::Result<T>`, an alias for `Result<T, oxipoint::Error>`.
`Error` is `#[non_exhaustive]`, so always keep a catch-all arm when matching.

| Variant       | Meaning                                                                                                                                        |
| ------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `Auth`        | The identity platform refused a token (`code` such as `invalid_grant`, plus a message).                                                        |
| `Config`      | Bad local input: certificate, URL, or a request aimed at a foreign host.                                                                       |
| `Http`        | Non-2xx service response: `status`, service `code`, `message`, `request_id`. A failed Graph copy is reported as status 500, code `copyFailed`. |
| `Transport`   | Network or TLS failure.                                                                                                                        |
| `Deserialize` | The body did not match the expected type.                                                                                                      |
| `Io`          | Local I/O: the browser flow's loopback listener, a writer given to `download_to`, or a copy wait that timed out.                               |

`Error::status()` returns the HTTP status for `Http` errors, and `None` for the rest.

Helpers save you from matching on raw statuses:

| Method              | True when                                                   |
| ------------------- | ----------------------------------------------------------- |
| `is_not_found()`    | HTTP 404                                                    |
| `is_unauthorized()` | HTTP 401, or token acquisition failed (`Error::Auth`)       |
| `is_forbidden()`    | HTTP 403                                                    |
| `is_conflict()`     | HTTP 409 or 412 (for example a failed etag guard)           |
| `is_throttled()`    | HTTP 429 or 503, after the SDK ran out of retries           |
| `code()`            | returns the service error code of an `Http` or `Auth` error |
| `request_id()`      | returns the service request id of an `Http` error           |

## Handling errors

```rust
use oxipoint::Error;

match graph.me().get().await {
    Ok(me) => println!("{:?}", me.display_name),
    Err(Error::Http { status: 403, code, .. }) => eprintln!("missing permission: {code:?}"),
    Err(Error::Http { status: 404, .. }) => eprintln!("not found"),
    Err(Error::Auth { code, message }) => eprintln!("sign-in failed ({code:?}): {message}"),
    Err(e) => eprintln!("failed: {e}"),
}
```

### Treat "not found" as a value

```rust
async fn find_list(sp: &SharePointClient, title: &str) -> oxipoint::Result<Option<oxipoint::sharepoint::ListInfo>> {
    match sp.list_by_title(title).get().await {
        Ok(list) => Ok(Some(list)),
        Err(e) if e.is_not_found() => Ok(None),
        Err(e) => Err(e),
    }
}
```

### Log the request id

`Http` errors carry `request_id`. Include it in logs and in support tickets with Microsoft: it is
the fastest way for them to trace a failure. `Error::request_id()` returns it directly.

### Built-in logging

Enable the `tracing` cargo feature (off by default) and the transport emits
[`tracing`](https://docs.rs/tracing) events:

| Level   | Event                                                                            |
| ------- | -------------------------------------------------------------------------------- |
| `debug` | each request attempt: method, host, path, status, attempt number, elapsed ms     |
| `warn`  | a retry (status, attempt, delay), and a final failure (status, code, request id) |

Query strings, headers and tokens are never logged.

The SDK only emits events; your application decides where they go by installing a subscriber:

```toml
oxipoint = { version = "0.0.1-beta7", features = ["tracing"] }
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

```rust
tracing_subscriber::fmt()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .init();
// RUST_LOG=oxipoint=debug cargo run   (output goes to stderr)
```

Use `tracing-appender` for files, or any other `tracing` subscriber, to send the events elsewhere.

## Retries and throttling

Microsoft 365 throttles heavy callers. oxipoint handles this for you:

- **429**, **503** and **504** are retried up to **3 times** by default.
- It waits for the `Retry-After` header when present, and **1 second** otherwise.
- After the last attempt, the final error is returned.
- `503` (without `Retry-After`) and `504` are **not** retried for non-idempotent requests, to
  avoid performing a write twice.

Tune the count:

```rust
let graph = GraphClient::builder(cred).max_retries(5).build();
```

For SharePoint, supply a `TransportConfig`:

```rust
use std::time::Duration;
use oxipoint::http::TransportConfig;

let sp = SharePointClient::with_config(
    site_url,
    cred,
    TransportConfig {
        max_retries: 6,
        default_retry_delay: Duration::from_secs(2),
        ..Default::default()
    },
)?;
```

### Inside batches

A throttled call inside a `$batch` is **not** retried automatically, because part of a batch cannot
be replayed safely. Inspect `BatchResult::retry_after()` and resend those calls in a new batch.
See [batching](../graph/sharing-search-batch.md).

## Credential safety

The bearer token is only sent to the client's own host. A request or continuation link that points
elsewhere fails with `Error::Config`. The deliberate exceptions are pre-authorized URLs, called
without a token: the upload session of large Graph uploads and the monitor URL of a Graph copy.

Secrets are wrapped in `Secret` and print as `[REDACTED]` in `Debug` output; errors never contain
them.
