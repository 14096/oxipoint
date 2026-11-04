# Cargo features

All features except `tracing` are enabled by default.

| Feature            | Enables                                            |
| ------------------ | -------------------------------------------------- |
| `graph`            | `oxipoint::graph`                                  |
| `sharepoint`       | `oxipoint::sharepoint`                             |
| `auth-password`    | `PasswordCredential`                               |
| `auth-device-code` | `DeviceCodeCredential`                             |
| `auth-browser`     | `BrowserCredential` (adds the `open` crate)        |
| `auth-certificate` | `CertificateCredential` (adds JWT and PFX crates)  |
| `tracing`          | Request, retry and failure events (off by default) |

```toml
oxipoint = { version = "0.0.1-beta7", default-features = false, features = ["sharepoint", "auth-certificate"] }
```

## Examples

| Goal                                       | Features                                  |
| ------------------------------------------ | ----------------------------------------- |
| Server job on SharePoint, certificate auth | `sharepoint`, `auth-certificate`          |
| CLI for users, any terminal                | `graph`, `sharepoint`, `auth-device-code` |
| Desktop app                                | `graph`, `auth-browser`                   |
| Test suite with a service account          | `graph`, `sharepoint`, `auth-password`    |
| Bring your own tokens                      | `graph` and/or `sharepoint` only          |

Always available, with no feature: `oxipoint::auth::{TokenCredential, StaticTokenCredential}`,
`oxipoint::fields`, `oxipoint::http` (generic builder, pager) and `oxipoint::Error`.

The `--no-default-features` build of the library compiles and is checked in CI.
