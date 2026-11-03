# Installation

## Requirements

- A recent stable Rust toolchain. The crate uses **edition 2024**.
- An async runtime. oxipoint is built on [tokio](https://tokio.rs).
- A Microsoft 365 tenant and an Entra ID app registration (see the [next page](app-registration.md)).

## Add the dependency

Add oxipoint from [crates.io](https://crates.io/crates/oxipoint). The current release is a pre-release, so the version must be given explicitly:

```toml
[dependencies]
oxipoint = { version = "0.0.1-beta6" }
tokio = { version = "1", features = ["macros", "rt-multi-thread", "fs"] }
serde_json = "1"   # handy for building request bodies with json!
reqwest = "0.13"   # only needed to name `reqwest::Method` for generic requests
```

## Trim what you do not use

Everything is enabled by default. A service that only talks to SharePoint with a certificate can
avoid pulling in the browser and other dependencies:

```toml
oxipoint = { version = "0.0.1-beta6", default-features = false, features = ["sharepoint", "auth-certificate"] }
```

See [Cargo features](../reference/features.md) for the full list.

## TLS

oxipoint uses `rustls`, so no system OpenSSL is needed. The same binary builds on Linux, macOS
and Windows.
