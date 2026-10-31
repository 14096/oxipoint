use std::{sync::Arc, time::Duration};

use reqwest::Method;
use serde::Deserialize;
use serde_json::Value;
use tokio::time::Instant;

use crate::{
    Error, Result,
    http::{RawRequest, Transport},
};

/// Where a copy stands.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CopyStatus {
    /// Accepted but not started.
    NotStarted,
    /// Running.
    InProgress,
    /// Finished; see [`CopyProgress::resource_id`].
    Completed,
    /// Failed.
    Failed,
    /// Any other value the service reports.
    Other(String),
}

impl CopyStatus {
    fn parse(s: &str) -> Self {
        match s {
            "notStarted" => CopyStatus::NotStarted,
            "inProgress" => CopyStatus::InProgress,
            "completed" => CopyStatus::Completed,
            "failed" => CopyStatus::Failed,
            other => CopyStatus::Other(other.to_owned()),
        }
    }
}

/// A snapshot of a copy operation.
#[derive(Debug, Clone, PartialEq)]
pub struct CopyProgress {
    /// Current status.
    pub status: CopyStatus,
    /// Percentage complete, when reported.
    pub percentage_complete: Option<f64>,
    /// Id of the new item once the copy completed.
    pub resource_id: Option<String>,
    /// Failure details, when the copy failed.
    pub error: Option<Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MonitorDoc {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    percentage_complete: Option<f64>,
    #[serde(default)]
    resource_id: Option<String>,
    #[serde(default)]
    error: Option<Value>,
}

/// An asynchronous copy started with `DriveItemRequest::copy`.
#[derive(Debug, Clone)]
pub struct CopyOperation {
    t: Arc<Transport>,
    url: String,
}

impl CopyOperation {
    pub(crate) fn new(t: Arc<Transport>, url: String) -> Self {
        Self { t, url }
    }

    /// The monitor URL returned by the service.
    pub fn monitor_url(&self) -> &str {
        &self.url
    }

    /// Reads the current state. The monitor URL is pre-authorized, so no token is sent.
    pub async fn status(&self) -> Result<CopyProgress> {
        let mut req = RawRequest::new(Method::GET, self.url.as_str());
        req.anonymous = true;
        let doc: MonitorDoc = self.t.send(&req).await?.json()?;
        Ok(CopyProgress {
            status: doc
                .status
                .as_deref()
                .map(CopyStatus::parse)
                .unwrap_or(CopyStatus::InProgress),
            percentage_complete: doc.percentage_complete,
            resource_id: doc.resource_id,
            error: doc.error,
        })
    }

    /// Polls every `interval` until the copy completes or fails, or `timeout` passes.
    ///
    /// A failed copy is returned as [`Error::Http`] with status 500 and code `copyFailed`;
    /// running out of time is an [`Error::Io`] of kind `TimedOut`. Unknown statuses count as
    /// still running.
    pub async fn wait(&self, interval: Duration, timeout: Duration) -> Result<CopyProgress> {
        let deadline = Instant::now() + timeout;
        loop {
            let p = self.status().await?;
            match p.status {
                CopyStatus::Completed => return Ok(p),
                CopyStatus::Failed => {
                    let message = p
                        .error
                        .as_ref()
                        .and_then(|e| e.get("message"))
                        .and_then(Value::as_str)
                        .unwrap_or("the copy failed")
                        .to_owned();
                    return Err(Error::Http {
                        status: 500,
                        code: Some("copyFailed".into()),
                        message,
                        request_id: None,
                    });
                }
                _ => {}
            }
            if Instant::now() + interval > deadline {
                return Err(Error::Io(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "timed out waiting for the copy to finish",
                )));
            }
            tokio::time::sleep(interval).await;
        }
    }
}
