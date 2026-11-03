# API reference

The complete type-level reference, generated from the source, is published on
[docs.rs/oxipoint](https://docs.rs/oxipoint).

Build it locally, including private items' links:

```sh
cargo doc --no-deps --open
```

Useful entry points:

| Module                | Contents                                                            |
| --------------------- | ------------------------------------------------------------------- |
| `oxipoint::auth`      | Credentials and the `TokenCredential` trait                         |
| `oxipoint::graph`     | `GraphClient` and its handles and models                            |
| `oxipoint::sharepoint`| `SharePointClient` and its handles and models                       |
| `oxipoint::fields`    | `Fields`, `FieldValue` and typed readers                            |
| `oxipoint::http`      | `RequestBuilder`, `Pager`, `QueryOptions`, `TransportConfig`        |
| `oxipoint::Error`     | The error type                                                      |

## Changelog

Release notes are in [`CHANGELOG.md`](https://github.com/14096/oxipoint/blob/main/CHANGELOG.md).
