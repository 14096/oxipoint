use serde::Serialize;

use super::{Column, ContentType, Permission};
use crate::{
    Result,
    http::{Body, RawRequest, encode_path},
};
use reqwest::Method;

collection!(
    /// A site's columns.
    ColumnsRequest => Column
);

creatable!(ColumnsRequest => Column);

impl ColumnsRequest {
    /// A column by id.
    pub fn by_id(&self, id: &str) -> ColumnRequest {
        ColumnRequest::new(self.t.clone(), format!("{}/{}", self.base, encode_path(id)))
    }
}

entity!(
    /// One site column.
    ColumnRequest => Column
);

collection!(
    /// A site's content types.
    ContentTypesRequest => ContentType
);

creatable!(ContentTypesRequest => ContentType);

impl ContentTypesRequest {
    /// A content type by id.
    pub fn by_id(&self, id: &str) -> ContentTypeRequest {
        ContentTypeRequest::new(self.t.clone(), format!("{}/{}", self.base, encode_path(id)))
    }
}

entity!(
    /// One site content type.
    ContentTypeRequest => ContentType
);

collection!(
    /// A site's permissions.
    PermissionsRequest => Permission
);

creatable!(PermissionsRequest => Permission);

impl PermissionsRequest {
    /// A permission by id.
    pub fn by_id(&self, id: &str) -> PermissionRequest {
        PermissionRequest::new(self.t.clone(), format!("{}/{}", self.base, encode_path(id)))
    }
}

entity!(
    /// One site permission.
    PermissionRequest => Permission
);

impl PermissionRequest {
    /// Updates the permission (for example its `roles`) and returns the result.
    pub async fn update<F: Serialize>(&self, changes: &F) -> Result<Permission> {
        let mut req = RawRequest::new(Method::PATCH, self.base.as_str());
        req.body = Body::Json(serde_json::to_value(changes)?);
        self.t.send(&req).await?.json()
    }
}

#[cfg(test)]
mod tests {
    use crate::graph::testutil::client;
    use serde_json::json;
    use wiremock::{
        Mock, ResponseTemplate,
        matchers::{body_json, method, path, query_param},
    };

    async fn get(s: &wiremock::MockServer, p: &str, body: serde_json::Value) {
        Mock::given(method("GET"))
            .and(path(p))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .expect(1)
            .mount(s)
            .await;
    }

    #[tokio::test]
    async fn columns_list_get_create_delete() {
        let (c, s) = client().await;
        get(
            &s,
            "/sites/s1/columns",
            json!({"value":[{"id":"c1","name":"Title"}]}),
        )
        .await;
        get(&s, "/sites/s1/columns/c%201", json!({"id":"c 1"})).await;
        Mock::given(method("POST"))
            .and(path("/sites/s1/columns"))
            .and(body_json(json!({"name":"N","text":{}})))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":"c2","name":"N"})))
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/sites/s1/columns/c2"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&s)
            .await;
        let cols = c.sites().by_id("s1").columns();
        assert_eq!(cols.list().await.unwrap()[0].name.as_deref(), Some("Title"));
        assert_eq!(cols.by_id("c 1").get().await.unwrap().id, "c 1");
        assert_eq!(
            cols.create(&json!({"name":"N","text":{}}))
                .await
                .unwrap()
                .id,
            "c2"
        );
        cols.by_id("c2").delete().await.unwrap();
    }

    #[tokio::test]
    async fn content_types_list_get_create_delete() {
        let (c, s) = client().await;
        get(
            &s,
            "/sites/s1/contentTypes",
            json!({"value":[{"id":"0x01"}]}),
        )
        .await;
        get(
            &s,
            "/sites/s1/contentTypes/0x0101",
            json!({"id":"0x0101","name":"Document"}),
        )
        .await;
        Mock::given(method("POST"))
            .and(path("/sites/s1/contentTypes"))
            .and(body_json(json!({"name":"Mine","base":{"id":"0x01"}})))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":"0x01AB"})))
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/sites/s1/contentTypes/0x01AB"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&s)
            .await;
        let cts = c.sites().by_id("s1").content_types();
        assert_eq!(cts.list().await.unwrap()[0].id, "0x01");
        assert_eq!(
            cts.by_id("0x0101").get().await.unwrap().name.as_deref(),
            Some("Document")
        );
        let made = cts
            .create(&json!({"name":"Mine","base":{"id":"0x01"}}))
            .await
            .unwrap();
        assert_eq!(made.id, "0x01AB");
        cts.by_id("0x01AB").delete().await.unwrap();
    }

    #[tokio::test]
    async fn permissions_crud() {
        let (c, s) = client().await;
        let next = format!("{}/sites/s1/permissions?p=2", s.uri());
        Mock::given(path("/sites/s1/permissions"))
            .and(query_param("p", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":[{"id":"b"}]})))
            .mount(&s)
            .await;
        Mock::given(method("GET"))
            .and(path("/sites/s1/permissions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                json!({"value":[{"id":"a","roles":["read"]}],"@odata.nextLink":next}),
            ))
            .mount(&s)
            .await;
        get(
            &s,
            "/sites/s1/permissions/a",
            json!({"id":"a","roles":["read"]}),
        )
        .await;
        Mock::given(method("POST"))
            .and(path("/sites/s1/permissions"))
            .and(body_json(json!({"roles":["write"]})))
            .respond_with(
                ResponseTemplate::new(201).set_body_json(json!({"id":"n","roles":["write"]})),
            )
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(method("PATCH"))
            .and(path("/sites/s1/permissions/a"))
            .and(body_json(json!({"roles":["write"]})))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"id":"a","roles":["write"]})),
            )
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/sites/s1/permissions/a"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&s)
            .await;
        let perms = c.sites().by_id("s1").permissions();
        let all = perms.list().await.unwrap();
        assert_eq!(
            (all.len(), all[0].roles.clone()),
            (2, vec!["read".to_owned()])
        );
        assert_eq!(perms.by_id("a").get().await.unwrap().id, "a");
        assert_eq!(
            perms.create(&json!({"roles":["write"]})).await.unwrap().id,
            "n"
        );
        let up = perms
            .by_id("a")
            .update(&json!({"roles":["write"]}))
            .await
            .unwrap();
        assert_eq!(up.roles, vec!["write".to_owned()]);
        let unit: () = perms.by_id("a").delete().await.unwrap();
        assert_eq!(unit, ());
    }
}
