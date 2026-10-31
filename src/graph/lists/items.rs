use crate::{
    Result,
    graph::*,
    http::{Body, Pager, QueryOptions, RawRequest, Transport, encode_path, query::query_methods},
};
use reqwest::Method;
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::sync::Arc;
use url::Url;

/// A list's items collection.
#[derive(Debug, Clone)]
pub struct ListItemsRequest {
    pub(super) t: Arc<Transport>,
    pub(super) base: String,
    pub(super) query: QueryOptions,
    pub(super) unindexed: bool,
}

impl ListItemsRequest {
    fn request(&self, path: &str) -> RawRequest {
        let mut req = request_with(path, &self.query);
        if self.unindexed {
            req.headers.push((
                "Prefer".into(),
                "HonorNonIndexedQueriesWarningMayFailRandomly".into(),
            ));
        }
        req
    }

    /// Filters on a column that is not indexed, which Graph only allows with an opt-in header.
    ///
    /// Sets `$filter` and sends `Prefer: HonorNonIndexedQueriesWarningMayFailRandomly`.
    /// On large lists such queries can fail.
    pub fn filter_unindexed(mut self, expr: impl Into<String>) -> Self {
        self.query = self.query.filter(expr);
        self.unindexed = true;
        self
    }

    /// Lists items, following paging links. Use `.expand("fields")` to include column values.
    pub async fn list(&self) -> Result<Vec<ListItem>> {
        self.pages().collect_all().await
    }

    /// Returns a pager.
    pub fn pages(&self) -> Pager<ListItem> {
        Pager::new(self.t.clone(), self.request(&self.base))
    }

    /// Creates an item from a serializable set of column values.
    pub async fn create<F: Serialize>(&self, fields: &F) -> Result<ListItem> {
        let mut req = RawRequest::new(Method::POST, self.base.as_str());
        req.body = Body::Json(json!({ "fields": serde_json::to_value(fields)? }));
        self.t.send(&req).await?.json()
    }

    /// Reads the items changed since `token` (everything when `None`), following paging links.
    ///
    /// The returned [`DeltaPage::next_token`] resumes the query on the next call.
    pub async fn delta(&self, token: Option<&str>) -> Result<DeltaPage> {
        let mut first = self.request(&format!("{}/delta", self.base));
        if let Some(t) = token {
            first.query.push(("token".into(), t.to_owned()));
        }
        let mut out = DeltaPage::default();
        let mut next = Some(first);
        while let Some(req) = next.take() {
            let doc: Value = self.t.send(&req).await?.json()?;
            if let Some(v) = doc.get("value") {
                out.items
                    .extend(serde_json::from_value::<Vec<ListItem>>(v.clone())?);
            }
            if let Some(link) = doc.get("@odata.nextLink").and_then(Value::as_str) {
                next = Some(RawRequest::new(Method::GET, link));
            } else if let Some(link) = doc.get("@odata.deltaLink").and_then(Value::as_str) {
                out.next_token = Url::parse(link).ok().and_then(|u| {
                    u.query_pairs()
                        .find(|(k, _)| k == "token")
                        .map(|(_, v)| v.into_owned())
                });
            }
        }
        Ok(out)
    }
}
query_methods!(ListItemsRequest);

/// One list item.
#[derive(Debug, Clone)]
pub struct ListItemRequest {
    pub(super) t: Arc<Transport>,
    pub(super) base: String,
    pub(super) query: QueryOptions,
}

impl ListItemRequest {
    /// Fetches the item.
    pub async fn get(&self) -> Result<ListItem> {
        get_json(&self.t, &self.base, &self.query).await
    }

    /// Updates the given columns and returns the resulting column values.
    pub async fn update<F: Serialize>(&self, fields: &F) -> Result<Map<String, Value>> {
        let mut req = RawRequest::new(Method::PATCH, format!("{}/fields", self.base));
        req.body = Body::Json(serde_json::to_value(fields)?);
        self.t.send(&req).await?.json()
    }

    /// Like [`update`](Self::update), but only if the item still has `etag` (sent as
    /// `If-Match`); a changed item fails with HTTP 412.
    pub async fn update_if_match<F: Serialize>(
        &self,
        fields: &F,
        etag: &str,
    ) -> Result<Map<String, Value>> {
        let mut req = RawRequest::new(Method::PATCH, format!("{}/fields", self.base));
        req.headers.push(("If-Match".into(), etag.to_owned()));
        req.body = Body::Json(serde_json::to_value(fields)?);
        self.t.send(&req).await?.json()
    }

    /// Deletes the item.
    pub async fn delete(&self) -> Result<()> {
        self.t
            .send(&RawRequest::new(Method::DELETE, self.base.as_str()))
            .await?;
        Ok(())
    }

    /// The drive item behind an item of a document library; use it for file operations.
    pub fn drive_item(&self) -> DriveItemRequest {
        DriveItemRequest::new(self.t.clone(), format!("{}/driveItem", self.base))
    }

    /// Activity statistics of the item.
    pub async fn analytics(&self) -> Result<ItemAnalytics> {
        get_json(&self.t, &format!("{}/analytics", self.base), &self.query).await
    }

    /// The activities on the item, across all pages.
    pub async fn activities(&self) -> Result<Vec<ItemActivity>> {
        pager(&self.t, &format!("{}/activities", self.base), &self.query)
            .collect_all()
            .await
    }

    /// Restores the version with the given id (such as `1.0`).
    pub async fn restore_version(&self, version_id: &str) -> Result<()> {
        self.t
            .send(&RawRequest::new(
                Method::POST,
                format!(
                    "{}/versions/{}/restoreVersion",
                    self.base,
                    encode_path(version_id)
                ),
            ))
            .await?;
        Ok(())
    }

    /// The versions of the item, across all pages.
    pub async fn versions(&self) -> Result<Vec<ItemVersion>> {
        pager(&self.t, &format!("{}/versions", self.base), &self.query)
            .collect_all()
            .await
    }
}
query_methods!(ListItemRequest);
