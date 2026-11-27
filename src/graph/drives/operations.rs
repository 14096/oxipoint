use reqwest::Method;
use serde_json::{Map, Value, json};

use super::{DriveItem, DriveItemRequest, DriveRequest, ItemPreview, ThumbnailSet};
use crate::{
    Result,
    graph::pager,
    http::{Body, QueryOptions, RawRequest},
};

impl DriveItemRequest {
    async fn post_action(
        &self,
        action: &str,
        body: Option<Value>,
    ) -> Result<crate::http::Response> {
        let mut req = RawRequest::new(Method::POST, format!("{}/{action}", self.base));
        if let Some(b) = body {
            req.body = Body::Json(b);
        }
        self.t.send(&req).await
    }

    /// Checks the file out so that nobody else can change it until it is checked in.
    pub async fn checkout(&self) -> Result<()> {
        self.post_action("checkout", None).await?;
        Ok(())
    }

    /// Checks the file in, optionally with a version `comment`.
    ///
    /// With `publish` the checked-in version is also published as a major version.
    pub async fn checkin(&self, comment: Option<&str>, publish: bool) -> Result<()> {
        let mut body = Map::new();
        if let Some(c) = comment {
            body.insert("comment".into(), c.into());
        }
        if publish {
            body.insert("checkInAs".into(), "published".into());
        }
        self.post_action("checkin", Some(Value::Object(body)))
            .await?;
        Ok(())
    }

    /// Gets URLs to embed a preview of the item. `page` and `zoom` apply to supported formats.
    pub async fn preview(&self, page: Option<&str>, zoom: Option<f64>) -> Result<ItemPreview> {
        let mut body = Map::new();
        if let Some(p) = page {
            body.insert("page".into(), p.into());
        }
        if let Some(z) = zoom {
            body.insert("zoom".into(), z.into());
        }
        self.post_action("preview", Some(Value::Object(body)))
            .await?
            .json()
    }

    /// Lists the thumbnail sets of the item.
    pub async fn thumbnails(&self) -> Result<Vec<ThumbnailSet>> {
        pager(
            &self.t,
            &format!("{}/thumbnails", self.base),
            &QueryOptions::new(),
        )
        .collect_all()
        .await
    }

    /// Follows the item and returns it.
    pub async fn follow(&self) -> Result<DriveItem> {
        self.post_action("follow", None).await?.json()
    }

    /// Stops following the item.
    pub async fn unfollow(&self) -> Result<()> {
        self.post_action("unfollow", None).await?;
        Ok(())
    }

    /// Restores a deleted item, optionally into another folder or under a new name.
    pub async fn restore(&self, parent_id: Option<&str>, name: Option<&str>) -> Result<DriveItem> {
        let mut body = Map::new();
        if let Some(p) = parent_id {
            body.insert("parentReference".into(), json!({ "id": p }));
        }
        if let Some(n) = name {
            body.insert("name".into(), n.into());
        }
        self.post_action("restore", Some(Value::Object(body)))
            .await?
            .json()
    }
}

impl DriveRequest {
    /// Searches the drive (`/root/search(q=...)`) for items matching `text` (file names, metadata and content).
    pub async fn find(&self, text: &str) -> Result<Vec<DriveItem>> {
        let q = crate::http::encode_path(&text.replace('\'', "''"));
        pager(
            &self.t,
            &format!("{}/root/search(q='{q}')", self.base),
            &self.query,
        )
        .collect_all()
        .await
    }
}

#[cfg(test)]
mod tests {
    use crate::graph::testutil::client;
    use serde_json::json;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{body_json, method, path, query_param},
    };

    async fn post(
        s: &MockServer,
        p: &str,
        body: Option<serde_json::Value>,
        resp: ResponseTemplate,
    ) {
        let b = Mock::given(method("POST")).and(path(p));
        let b = match body {
            Some(v) => b.and(body_json(v)),
            None => b,
        };
        b.respond_with(resp).expect(1).mount(s).await;
    }

    #[tokio::test]
    async fn checkout_and_checkin() {
        let (c, s) = client().await;
        post(
            &s,
            "/drives/d/items/i/checkout",
            None,
            ResponseTemplate::new(204),
        )
        .await;
        post(
            &s,
            "/drives/d/items/i/checkin",
            Some(json!({"comment":"done","checkInAs":"published"})),
            ResponseTemplate::new(204),
        )
        .await;
        let item = c.drive("d").item("i");
        item.checkout().await.unwrap();
        item.checkin(Some("done"), true).await.unwrap();
    }

    #[tokio::test]
    async fn checkin_without_options_sends_an_empty_object() {
        let (c, s) = client().await;
        post(
            &s,
            "/drives/d/items/i/checkin",
            Some(json!({})),
            ResponseTemplate::new(204),
        )
        .await;
        c.drive("d").item("i").checkin(None, false).await.unwrap();
    }

    #[tokio::test]
    async fn preview_and_thumbnails() {
        let (c, s) = client().await;
        post(
            &s,
            "/drives/d/items/i/preview",
            Some(json!({"page":"2","zoom":1.5})),
            ResponseTemplate::new(200).set_body_json(
                json!({"getUrl":"https://p/get","postUrl":"https://p/post","postParameters":"a=b"}),
            ),
        )
        .await;
        Mock::given(method("GET"))
            .and(path("/drives/d/items/i/thumbnails"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":[
                {"id":"0","small":{"height":96,"width":96,"url":"https://t/s"},"large":{"url":"https://t/l"}}
            ]})))
            .mount(&s)
            .await;
        let item = c.drive("d").item("i");
        let p = item.preview(Some("2"), Some(1.5)).await.unwrap();
        assert_eq!(p.get_url.as_deref(), Some("https://p/get"));
        let t = item.thumbnails().await.unwrap();
        assert_eq!(t[0].small.as_ref().unwrap().width, Some(96));
        assert_eq!(t[0].medium, None);
    }

    #[tokio::test]
    async fn follow_unfollow_and_restore() {
        let (c, s) = client().await;
        post(
            &s,
            "/drives/d/items/i/follow",
            None,
            ResponseTemplate::new(200).set_body_json(json!({"id":"i"})),
        )
        .await;
        post(
            &s,
            "/drives/d/items/i/unfollow",
            None,
            ResponseTemplate::new(204),
        )
        .await;
        post(
            &s,
            "/drives/d/items/i/restore",
            Some(json!({"parentReference":{"id":"p"},"name":"back.txt"})),
            ResponseTemplate::new(200).set_body_json(json!({"id":"i","name":"back.txt"})),
        )
        .await;
        let item = c.drive("d").item("i");
        assert_eq!(item.follow().await.unwrap().id, "i");
        item.unfollow().await.unwrap();
        let r = item.restore(Some("p"), Some("back.txt")).await.unwrap();
        assert_eq!(r.name.as_deref(), Some("back.txt"));
    }

    #[tokio::test]
    async fn find_in_a_drive_escapes_the_query() {
        let (c, s) = client().await;
        Mock::given(method("GET"))
            .and(path("/drives/d/root/search(q='it%27%27s%20a%20budget')"))
            .and(query_param("$top", "5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":[{"id":"x"}]})))
            .expect(1)
            .mount(&s)
            .await;
        let hits = c.drive("d").top(5).find("it's a budget").await.unwrap();
        assert_eq!(hits[0].id, "x");
    }
}
