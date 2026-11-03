# Interactive browser

Opens the system browser, listens on `http://localhost:<port>` for the redirect, and exchanges the
returned code using **PKCE**. The `state` value is verified, and the wait times out after
5 minutes.

**Best for:** desktop tools and developer utilities.

**App registration:** platform *Mobile and desktop* with redirect URI `http://localhost`; enable
public client flows.

```rust
use std::time::Duration;
use oxipoint::auth::BrowserCredential;

let cred = BrowserCredential::new("contoso.onmicrosoft.com", "<client-id>")
    .with_port(8400)                         // optional: fixed port (default: random free port)
    .with_timeout(Duration::from_secs(120)); // optional
```

## Options

| Method                | Purpose                                                                       |
| --------------------- | ----------------------------------------------------------------------------- |
| `with_port(u16)`      | Use a fixed loopback port instead of a random free one                        |
| `with_timeout(d)`     | How long to wait for the user (default 5 minutes)                             |
| `with_url_opener(f)`  | Replace how the sign-in URL is opened, e.g. to print it or open a specific browser |
| `with_authority(url)` | Target a national cloud                                                       |

## Notes

- A random port works with the `http://localhost` redirect URI because Entra ignores the port for
  loopback addresses. Use `with_port` only if a firewall or policy requires a known port.
- If the user closes the browser, the call fails with `Error::Auth` (code `timeout`) when the timeout is reached.
- The refresh token is kept, so a later request for another scope (Graph, then SharePoint) does
  not prompt again.
