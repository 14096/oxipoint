use reqwest::Method;
use serde::Serialize;

use super::{SitePage, WebPart};
use crate::{
    Result,
    http::{Body, RawRequest, encode_path},
};

const CAST: &str = "microsoft.graph.sitePage";

collection!(
    /// A site's pages.
    PagesRequest => SitePage
);

creatable!(PagesRequest => SitePage);

impl PagesRequest {
    /// A page by id.
    pub fn by_id(&self, id: &str) -> PageRequest {
        PageRequest::new(
            self.t.clone(),
            format!("{}/{}/{CAST}", self.base, encode_path(id)),
        )
    }
}

entity!(
    /// One site page.
    PageRequest => SitePage
);

impl PageRequest {
    /// Updates the page's editable properties and returns the result.
    pub async fn update<F: Serialize>(&self, changes: &F) -> Result<SitePage> {
        let mut req = RawRequest::new(Method::PATCH, self.base.as_str());
        req.body = Body::Json(serde_json::to_value(changes)?);
        self.t.send(&req).await?.json()
    }

    /// Publishes the page's latest draft.
    pub async fn publish(&self) -> Result<()> {
        let req = RawRequest::new(Method::POST, format!("{}/publish", self.base));
        self.t.send(&req).await?;
        Ok(())
    }

    /// The page's web parts.
    pub fn web_parts(&self) -> WebPartsRequest {
        WebPartsRequest::new(self.t.clone(), format!("{}/webParts", self.base))
    }
}

collection!(
    /// A page's web parts.
    WebPartsRequest => WebPart
);

impl WebPartsRequest {
    /// A web part by id.
    pub fn by_id(&self, id: &str) -> WebPartRequest {
        WebPartRequest::new(self.t.clone(), format!("{}/{}", self.base, encode_path(id)))
    }
}

entity!(
    /// One web part of a page.
    WebPartRequest => WebPart
);

#[cfg(test)]
mod tests {
    use crate::graph::{NewSitePage, testutil::client};
    use serde_json::json;
    use wiremock::{
        Mock, ResponseTemplate,
        matchers::{body_json, method, path, query_param},
    };

    const SITE: &str = "/sites/contoso.sharepoint.com:/sites/team";

    #[tokio::test]
    async fn lists_pages_across_pages() {
        let (c, s) = client().await;
        let next = format!("{}/sites/x/pages?p=2", s.uri());
        Mock::given(method("GET"))
            .and(path(SITE.to_owned() + "/pages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value": [{"id": "1", "title": "A"}], "@odata.nextLink": next
            })))
            .mount(&s)
            .await;
        Mock::given(method("GET"))
            .and(path("/sites/x/pages"))
            .and(query_param("p", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value": [{"id": "2"}]})))
            .mount(&s)
            .await;
        let pages = c
            .sites()
            .by_path("contoso.sharepoint.com", "/sites/team/")
            .pages()
            .list()
            .await
            .unwrap();
        assert_eq!(
            pages.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
            ["1", "2"]
        );
    }

    #[tokio::test]
    async fn reads_page_with_canvas_layout() {
        let (c, s) = client().await;
        Mock::given(method("GET"))
            .and(path("/sites/root/pages/p%201/microsoft.graph.sitePage"))
            .and(query_param("$expand", "canvasLayout"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "p 1", "title": "Home", "publishingState": {"level": "published"},
                "canvasLayout": {"horizontalSections": [{"layout": "oneColumn", "columns": [
                    {"width": 12, "webparts": [{"id": "w", "@odata.type": "#microsoft.graph.textWebPart", "innerHtml": "<p>hi</p>"}]}
                ]}]}
            })))
            .mount(&s)
            .await;
        let p = c
            .sites()
            .root()
            .pages()
            .by_id("p 1")
            .expand("canvasLayout")
            .get()
            .await
            .unwrap();
        let wp = &p.canvas_layout.unwrap().horizontal_sections[0].columns[0].webparts[0];
        assert_eq!(wp.inner_html.as_deref(), Some("<p>hi</p>"));
        assert_eq!(
            p.publishing_state.unwrap().level.as_deref(),
            Some("published")
        );
    }

    #[tokio::test]
    async fn missing_page_is_an_error() {
        let (c, s) = client().await;
        Mock::given(method("GET"))
            .and(path("/sites/root/pages/nope/microsoft.graph.sitePage"))
            .respond_with(
                ResponseTemplate::new(404)
                    .set_body_json(json!({"error": {"code": "itemNotFound", "message": "no"}})),
            )
            .mount(&s)
            .await;
        assert!(c.sites().root().pages().by_id("nope").get().await.is_err());
    }

    #[tokio::test]
    async fn creates_updates_and_deletes() {
        let (c, s) = client().await;
        Mock::given(method("POST"))
            .and(path("/sites/root/pages"))
            .and(body_json(json!({
                "@odata.type": "#microsoft.graph.sitePage", "name": "Home.aspx", "title": "Home"
            })))
            .respond_with(
                ResponseTemplate::new(201).set_body_json(json!({"id": "n", "name": "Home.aspx"})),
            )
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(method("PATCH"))
            .and(path("/sites/root/pages/n/microsoft.graph.sitePage"))
            .and(body_json(json!({"title": "New"})))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"id": "n", "title": "New"})),
            )
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/sites/root/pages/n/microsoft.graph.sitePage"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&s)
            .await;
        let pages = c.sites().root().pages();
        let created = pages
            .create(&NewSitePage::new("Home.aspx").title("Home"))
            .await
            .unwrap();
        assert_eq!(created.id, "n");
        let page = pages.by_id("n");
        assert_eq!(
            page.update(&json!({"title": "New"}))
                .await
                .unwrap()
                .title
                .as_deref(),
            Some("New")
        );
        page.delete().await.unwrap();
    }

    #[tokio::test]
    async fn publishes() {
        let (c, s) = client().await;
        Mock::given(method("POST"))
            .and(path("/sites/root/pages/n/microsoft.graph.sitePage/publish"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&s)
            .await;
        c.sites().root().pages().by_id("n").publish().await.unwrap();
    }

    #[tokio::test]
    async fn lists_and_reads_web_parts() {
        let (c, s) = client().await;
        Mock::given(method("GET"))
            .and(path("/sites/root/pages/n/microsoft.graph.sitePage/webParts"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value": [{"id": "w1", "@odata.type": "#microsoft.graph.standardWebPart", "webPartType": "abc"}]
            })))
            .mount(&s)
            .await;
        Mock::given(method("GET"))
            .and(path(
                "/sites/root/pages/n/microsoft.graph.sitePage/webParts/w1",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "w1"})))
            .mount(&s)
            .await;
        let parts = c.sites().root().pages().by_id("n").web_parts();
        let all = parts.list().await.unwrap();
        assert_eq!(all[0].extra["webPartType"], "abc");
        assert_eq!(
            parts.by_id("w1").get().await.unwrap().id.as_deref(),
            Some("w1")
        );
    }
}
