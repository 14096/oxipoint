//! Shared HTTP plumbing: transport with auth and retry, generic requests, paging.

mod pager;
pub(crate) mod query;
mod request;
mod transport;

pub use pager::Pager;
pub use query::QueryOptions;
pub use request::{Body, RawRequest, RequestBuilder, Response};
pub use transport::{Transport, TransportConfig};

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};

const PATH_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~')
    .remove(b'/');

/// Percent-encodes a path, keeping `/` separators.
pub fn encode_path(path: &str) -> String {
    utf8_percent_encode(path, PATH_SET).to_string()
}
