use crate::graph::*;

collection!(
    /// A list's permissions.
    ListPermissionsRequest => Permission
);
creatable!(ListPermissionsRequest => Permission);

#[cfg(test)]
mod tests {
    use crate::graph::testutil::client;
    use serde_json::json;
    use wiremock::{
        Mock, ResponseTemplate,
        matchers::{body_json, method, path},
    };

    #[tokio::test]
    async fn list_and_create_permissions() {
        let (c, s) = client().await;
        Mock::given(method("GET"))
            .and(path("/sites/s1/lists/l1/permissions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"value":[{"id":"p1","roles":["read"]}]})),
            )
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(method("POST"))
            .and(path("/sites/s1/lists/l1/permissions"))
            .and(body_json(json!({"roles":["write"]})))
            .respond_with(
                ResponseTemplate::new(201).set_body_json(json!({"id":"p2","roles":["write"]})),
            )
            .expect(1)
            .mount(&s)
            .await;
        let perms = c.sites().by_id("s1").list("l1").permissions();
        assert_eq!(perms.list().await.unwrap()[0].id, "p1");
        assert_eq!(
            perms.create(&json!({"roles":["write"]})).await.unwrap().id,
            "p2"
        );
    }

    #[tokio::test]
    async fn all_sites_across_geographies() {
        let (c, s) = client().await;
        Mock::given(method("GET"))
            .and(path("/sites/getAllSites"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"value":[{"id":"a"},{"id":"b"}]})),
            )
            .expect(1)
            .mount(&s)
            .await;
        assert_eq!(c.sites().all().await.unwrap().len(), 2);
    }
}
