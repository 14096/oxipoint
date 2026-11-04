//! A fluent, type-safe SDK for SharePoint REST and Microsoft Graph.
//!
//! # Overview
//! * [`auth`] — credentials implementing [`auth::TokenCredential`]: user/password,
//!   certificate, interactive browser and device code (each behind a cargo feature).
//! * [`graph`] — typed Microsoft Graph client (feature `graph`).
//! * [`sharepoint`] — typed SharePoint REST client (feature `sharepoint`).
//! * [`http`] — the shared transport and the generic [`http::RequestBuilder`] used to
//!   call endpoints that have no typed wrapper yet.
//!
//! # Example
//! ```no_run
//! # async fn run() -> oxipoint::Result<()> {
//! use oxipoint::auth::PasswordCredential;
//! use oxipoint::graph::GraphClient;
//!
//! let cred = PasswordCredential::new("tenant-id", "client-id", "user@contoso.com", "secret");
//! let graph = GraphClient::new(cred);
//! let me = graph.me().get().await?;
//! println!("{:?}", me.display_name);
//!
//! // Endpoints without a typed wrapper:
//! let raw: serde_json::Value = graph.request(reqwest::Method::GET, "/me/messages")
//!     .query("$top", "5")
//!     .send_json()
//!     .await?;
//! # let _ = raw; Ok(()) }
//! ```
//!
//! # Extending
//! Implement [`auth::TokenCredential`] to add an authentication flow; add new API areas as
//! modules that wrap an `Arc<http::Transport>`.

#![warn(missing_docs)]

pub mod auth;
mod error;
pub mod fields;
pub mod filter;
pub mod http;
mod secret;

#[cfg(feature = "graph")]
pub mod graph;
#[cfg(feature = "sharepoint")]
pub mod sharepoint;

pub use error::{Error, Result};
pub use secret::Secret;
