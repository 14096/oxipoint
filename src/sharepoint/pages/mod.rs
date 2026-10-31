mod models;

pub use models::{NewSitePage, SitePage};

use serde::Serialize;

use crate::{
    Result,
    http::Body,
    sharepoint::inner::{json_body, post, post_action},
};

collection!(
    /// The modern pages of a web.
    PagesRequest => SitePage
);

impl PagesRequest {
    /// A page by item id.
    pub fn by_id(&self, id: u32) -> PageRequest {
        PageRequest::new(self.inner.clone(), format!("{}({id})", self.base))
    }

    /// Creates a page from a definition and returns it.
    pub async fn create(&self, page: &NewSitePage) -> Result<SitePage> {
        post(&self.inner, self.base.clone(), json_body(page)?).await
    }
}

entity!(
    /// One modern page.
    PageRequest => SitePage
);

impl PageRequest {
    /// Saves the given properties (for example `Title` or `CanvasContent1`) as a draft and
    /// returns the page as it is afterwards.
    pub async fn update<F: Serialize>(&self, changes: &F) -> Result<SitePage> {
        post_action(
            &self.inner,
            format!("{}/savepage", self.base),
            json_body(changes)?,
        )
        .await?;
        self.get().await
    }

    /// Publishes the page's latest draft.
    pub async fn publish(&self) -> Result<()> {
        post_action(&self.inner, format!("{}/publish", self.base), Body::None).await
    }

    /// Deletes the page.
    pub async fn delete(&self) -> Result<()> {
        self.inner
            .tunnel("DELETE", self.base.clone(), Body::None)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::sharepoint::{
        NewSitePage,
        testutil::{client, mock_get, mock_write, mount_digest},
    };
    use serde_json::json;
    use wiremock::{
        Mock, ResponseTemplate,
        matchers::{method, path, query_param},
    };

    #[tokio::test]
    async fn lists_pages_across_pages() {
        let (c, s) = client().await;
        let next = format!("{}/_api/sitepages/pages?p=2", s.uri());
        Mock::given(method("GET"))
            .and(path("/_api/sitepages/pages"))
            .and(query_param("$top", "1"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(
                    json!({"value": [{"Id": 1, "Title": "A"}], "odata.nextLink": next}),
                ),
            )
            .mount(&s)
            .await;
        Mock::given(method("GET"))
            .and(path("/_api/sitepages/pages"))
            .and(query_param("p", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value": [{"Id": 2}]})))
            .mount(&s)
            .await;
        let pages = c.web().pages().top(1).list().await.unwrap();
        assert_eq!(pages.iter().map(|p| p.id).collect::<Vec<_>>(), [1, 2]);
    }

    #[tokio::test]
    async fn reads_page_with_canvas_content() {
        let (c, s) = client().await;
        mock_get(
            &s,
            "/_api/sitepages/pages(7)",
            json!({"Id": 7, "FileName": "Home.aspx", "Title": "Home",
                   "PageLayoutType": "Home", "CanvasContent1": "[{}]"}),
        )
        .await;
        let p = c.web().pages().by_id(7).get().await.unwrap();
        assert_eq!(p.file_name.as_deref(), Some("Home.aspx"));
        assert_eq!(p.canvas_content.as_deref(), Some("[{}]"));
    }

    #[tokio::test]
    async fn missing_page_is_an_error() {
        let (c, s) = client().await;
        Mock::given(method("GET"))
            .and(path("/_api/sitepages/pages(404)"))
            .respond_with(ResponseTemplate::new(404).set_body_json(
                json!({"odata.error": {"code": "-1, Microsoft.SharePoint.SPException",
                       "message": {"lang": "en-US", "value": "not found"}}}),
            ))
            .mount(&s)
            .await;
        assert!(c.web().pages().by_id(404).get().await.is_err());
    }

    #[tokio::test]
    async fn creates_updates_publishes_and_deletes() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        mock_write(
            &s,
            "/_api/sitepages/pages",
            None,
            Some(json!({
                "__metadata": {"type": "SP.Publishing.SitePage"},
                "Name": "News.aspx", "Title": "News"
            })),
            json!({"Id": 3, "FileName": "News.aspx", "Title": "News"}),
        )
        .await;
        mock_write(
            &s,
            "/_api/sitepages/pages(3)/savepage",
            None,
            Some(json!({"Title": "Latest"})),
            json!({"value": true}),
        )
        .await;
        mock_get(
            &s,
            "/_api/sitepages/pages(3)",
            json!({"Id": 3, "Title": "Latest"}),
        )
        .await;
        mock_write(
            &s,
            "/_api/sitepages/pages(3)/publish",
            None,
            None,
            json!({"value": true}),
        )
        .await;
        mock_write(
            &s,
            "/_api/sitepages/pages(3)",
            Some("DELETE"),
            None,
            json!(null),
        )
        .await;
        let pages = c.web().pages();
        let created = pages
            .create(&NewSitePage::new("News.aspx").title("News"))
            .await
            .unwrap();
        assert_eq!(created.id, 3);
        let page = pages.by_id(3);
        let updated = page.update(&json!({"Title": "Latest"})).await.unwrap();
        assert_eq!(updated.title.as_deref(), Some("Latest"));
        page.publish().await.unwrap();
        page.delete().await.unwrap();
    }
}
