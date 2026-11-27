mod models;

pub use models::{DirectoryObject, Group};

use reqwest::Method;
use serde::Serialize;
use serde_json::json;

use crate::{
    Result,
    graph::*,
    http::{Body, RawRequest, encode_path},
};

collection!(
    /// `/groups` — the groups of the tenant.
    GroupsRequest => Group
);
creatable!(GroupsRequest => Group);

impl GroupsRequest {
    /// A group by object id.
    pub fn by_id(&self, id: &str) -> GroupRequest {
        GroupRequest::new(self.t.clone(), format!("{}/{}", self.base, encode_path(id)))
    }
}

entity!(
    /// `/groups/{id}` — one group.
    GroupRequest => Group
);

impl GroupRequest {
    /// Updates the given properties of the group.
    pub async fn update<F: Serialize>(&self, changes: &F) -> Result<()> {
        let mut req = RawRequest::new(Method::PATCH, self.base.as_str());
        req.body = Body::Json(serde_json::to_value(changes)?);
        self.t.send(&req).await?;
        Ok(())
    }

    /// The direct members of the group.
    pub fn members(&self) -> DirectoryObjectsRequest {
        DirectoryObjectsRequest::new(self.t.clone(), format!("{}/members", self.base))
    }

    /// The owners of the group.
    pub fn owners(&self) -> DirectoryObjectsRequest {
        DirectoryObjectsRequest::new(self.t.clone(), format!("{}/owners", self.base))
    }

    /// The groups this group is a member of.
    pub fn member_of(&self) -> DirectoryObjectsRequest {
        DirectoryObjectsRequest::new(self.t.clone(), format!("{}/memberOf", self.base))
    }

    /// The team site of a Microsoft 365 group.
    pub fn site(&self) -> SiteRequest {
        SiteRequest::new(self.t.clone(), format!("{}/sites/root", self.base))
    }

    /// The default document library of a Microsoft 365 group.
    pub fn drive(&self) -> DriveRequest {
        DriveRequest::new(self.t.clone(), format!("{}/drive", self.base))
    }
}

collection!(
    /// Directory objects related to a group or user (members, owners, memberships).
    DirectoryObjectsRequest => DirectoryObject
);

impl DirectoryObjectsRequest {
    /// Adds the directory object `id` (user, group, ...) to this members or owners collection.
    pub async fn add(&self, id: &str) -> Result<()> {
        let odata_id = format!("{}/directoryObjects/{}", self.t.base_url(), encode_path(id));
        let mut req = RawRequest::new(Method::POST, format!("{}/$ref", self.base));
        req.body = Body::Json(json!({ "@odata.id": odata_id }));
        self.t.send(&req).await?;
        Ok(())
    }

    /// Removes the directory object `id` from this members or owners collection.
    pub async fn remove(&self, id: &str) -> Result<()> {
        let req = RawRequest::new(
            Method::DELETE,
            format!("{}/{}/$ref", self.base, encode_path(id)),
        );
        self.t.send(&req).await?;
        Ok(())
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

    async fn mock(s: &MockServer, m: &str, p: &str, status: u16, body: serde_json::Value) {
        let r = if body.is_null() {
            ResponseTemplate::new(status)
        } else {
            ResponseTemplate::new(status).set_body_json(body)
        };
        Mock::given(method(m))
            .and(path(p))
            .respond_with(r)
            .expect(1)
            .mount(s)
            .await;
    }

    #[tokio::test]
    async fn list_get_create_update_delete() {
        let (c, s) = client().await;
        Mock::given(method("GET"))
            .and(path("/groups"))
            .and(query_param("$filter", "startswith(displayName,'T')"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":[
                {"id":"g1","displayName":"Team","groupTypes":["Unified"],"mailEnabled":true}
            ]})))
            .mount(&s)
            .await;
        mock(
            &s,
            "GET",
            "/groups/g1",
            200,
            json!({"id":"g1","visibility":"Private"}),
        )
        .await;
        Mock::given(method("POST"))
            .and(path("/groups"))
            .and(body_json(json!({"displayName":"New"})))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":"g2"})))
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(method("PATCH"))
            .and(path("/groups/g1"))
            .and(body_json(json!({"description":"d"})))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&s)
            .await;
        mock(&s, "DELETE", "/groups/g1", 204, json!(null)).await;

        let all = c
            .groups()
            .filter("startswith(displayName,'T')")
            .list()
            .await
            .unwrap();
        assert!(all[0].is_microsoft_365());
        assert_eq!(all[0].mail_enabled, Some(true));
        let g = c.group("g1");
        assert_eq!(
            g.get().await.unwrap().visibility.as_deref(),
            Some("Private")
        );
        assert_eq!(
            c.groups()
                .create(&json!({"displayName":"New"}))
                .await
                .unwrap()
                .id,
            "g2"
        );
        g.update(&json!({"description":"d"})).await.unwrap();
        g.delete().await.unwrap();
    }

    #[tokio::test]
    async fn members_owners_and_membership_changes() {
        let (c, s) = client().await;
        mock(
            &s,
            "GET",
            "/groups/g1/members",
            200,
            json!({"value":[{"id":"u1","@odata.type":"#microsoft.graph.user","userPrincipalName":"a@x.com"}]}),
        )
        .await;
        mock(
            &s,
            "GET",
            "/groups/g1/owners",
            200,
            json!({"value":[{"id":"u2"}]}),
        )
        .await;
        mock(&s, "GET", "/groups/g1/memberOf", 200, json!({"value":[]})).await;
        Mock::given(method("POST"))
            .and(path("/groups/g1/members/$ref"))
            .and(body_json(
                json!({"@odata.id": format!("{}/directoryObjects/u3", s.uri())}),
            ))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&s)
            .await;
        mock(&s, "DELETE", "/groups/g1/owners/u2/$ref", 204, json!(null)).await;

        let g = c.group("g1");
        let members = g.members().list().await.unwrap();
        assert_eq!(
            members[0].odata_type.as_deref(),
            Some("#microsoft.graph.user")
        );
        assert_eq!(g.owners().list().await.unwrap()[0].id, "u2");
        assert!(g.member_of().list().await.unwrap().is_empty());
        g.members().add("u3").await.unwrap();
        g.owners().remove("u2").await.unwrap();
    }

    #[tokio::test]
    async fn team_site_and_drive() {
        let (c, s) = client().await;
        mock(
            &s,
            "GET",
            "/groups/g1/sites/root",
            200,
            json!({"id":"site1"}),
        )
        .await;
        mock(
            &s,
            "GET",
            "/groups/g1/sites/root/lists",
            200,
            json!({"value":[]}),
        )
        .await;
        mock(&s, "GET", "/groups/g1/drive", 200, json!({"id":"d1"})).await;
        let g = c.group("g1");
        assert_eq!(g.site().get().await.unwrap().id, "site1");
        g.site().lists().list().await.unwrap();
        assert_eq!(g.drive().get().await.unwrap().id, "d1");
    }
}
