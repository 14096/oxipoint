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
        Err(e) if e.status() == Some(404) => Ok(None),
        Err(e) => Err(e),
    }
}
```

### Log the request id

`Http` errors carry `request_id`. Include it in logs and in support tickets with Microsoft: it is
the fastest way for them to trace a failure.

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
