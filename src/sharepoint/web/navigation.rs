use std::sync::Arc;

use serde::Serialize;
use serde_json::json;

use super::{json_body, post};
use crate::{
    Result,
    http::Body,
    sharepoint::{Inner, NavigationNode},
};

/// `/web/navigation` — the top navigation bar and quick launch.
#[derive(Debug, Clone)]
pub struct NavigationRequest {
    inner: Arc<Inner>,
    base: String,
}

impl NavigationRequest {
    pub(crate) fn new(inner: Arc<Inner>, base: String) -> Self {
        Self { inner, base }
    }

    /// The top navigation bar.
    pub fn top(&self) -> NavigationNodesRequest {
        NavigationNodesRequest::new(
            self.inner.clone(),
            format!("{}/topnavigationbar", self.base),
        )
    }

    /// The quick launch.
    pub fn quick_launch(&self) -> NavigationNodesRequest {
        NavigationNodesRequest::new(self.inner.clone(), format!("{}/quicklaunch", self.base))
    }

    /// A node by id.
    pub fn node(&self, id: i64) -> NavigationNodeRequest {
        NavigationNodeRequest::new(
            self.inner.clone(),
            format!("{}/getnodebyid({id})", self.base),
        )
    }
}

collection!(
    /// A collection of navigation nodes.
    NavigationNodesRequest => NavigationNode
);

impl NavigationNodesRequest {
    /// Adds a node with a title and URL.
    pub async fn add(&self, title: &str, url: &str) -> Result<NavigationNode> {
        post(
            &self.inner,
            self.base.clone(),
            Body::Json(json!({ "Title": title, "Url": url })),
        )
        .await
    }
}

entity!(
    /// One navigation node.
    NavigationNodeRequest => NavigationNode
);

impl NavigationNodeRequest {
    /// The node's children.
    pub fn children(&self) -> NavigationNodesRequest {
        NavigationNodesRequest::new(self.inner.clone(), format!("{}/children", self.base))
    }

    /// Updates the given properties (MERGE).
    pub async fn update<F: Serialize>(&self, properties: &F) -> Result<()> {
        self.inner
            .tunnel("MERGE", self.base.clone(), json_body(properties)?)
            .await?;
        Ok(())
    }

    /// Deletes the node.
    pub async fn delete(&self) -> Result<()> {
        self.inner
            .tunnel("DELETE", self.base.clone(), Body::None)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::sharepoint::testutil::{client, mock_get, mock_write, mount_digest};
    use serde_json::json;

    #[tokio::test]
    async fn lists_and_reads_nodes() {
        let (c, s) = client().await;
        mock_get(
            &s,
            "/_api/web/navigation/topnavigationbar",
            json!({"value":[{"Id":1,"Title":"Home"}]}),
        )
        .await;
        mock_get(
            &s,
            "/_api/web/navigation/quicklaunch",
            json!({"value":[{"Id":2},{"Id":3}]}),
        )
        .await;
        mock_get(
            &s,
            "/_api/web/navigation/getnodebyid(2)",
            json!({"Id":2,"Title":"Docs"}),
        )
        .await;
        mock_get(
            &s,
            "/_api/web/navigation/getnodebyid(2)/children",
            json!({"value":[{"Id":4}]}),
        )
        .await;
        let nav = c.web().navigation();
        assert_eq!(
            nav.top().list().await.unwrap()[0].title.as_deref(),
            Some("Home")
        );
        assert_eq!(nav.quick_launch().list().await.unwrap().len(), 2);
        assert_eq!(
            nav.node(2).get().await.unwrap().title.as_deref(),
            Some("Docs")
        );
        assert_eq!(nav.node(2).children().list().await.unwrap()[0].id, 4);
    }

    #[tokio::test]
    async fn add_update_delete_node() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        mock_write(
            &s,
            "/_api/web/navigation/topnavigationbar",
            None,
            Some(json!({"Title":"T","Url":"/x"})),
            json!({"Id":7,"Title":"T","Url":"/x"}),
        )
        .await;
        mock_write(
            &s,
            "/_api/web/navigation/getnodebyid(7)",
            Some("MERGE"),
            Some(json!({"Title":"U"})),
            json!(null),
        )
        .await;
        mock_write(
            &s,
            "/_api/web/navigation/getnodebyid(8)",
            Some("DELETE"),
            None,
            json!(null),
        )
        .await;
        let nav = c.web().navigation();
        assert_eq!(nav.top().add("T", "/x").await.unwrap().id, 7);
        nav.node(7).update(&json!({"Title":"U"})).await.unwrap();
        nav.node(8).delete().await.unwrap();
    }
}
