mod models;

pub use models::User;

use std::sync::Arc;

use bytes::Bytes;
use reqwest::Method;
use serde_json::json;

use crate::graph::*;
use crate::{
    Result,
    http::{Pager, QueryOptions, RawRequest, Transport, query::query_methods},
};

/// Adds directory relationships and the photo to a handle with a `base_path()` method.
macro_rules! user_relations {
    ($t:ty) => {
        impl $t {
            /// The user's manager.
            pub async fn manager(&self) -> Result<DirectoryObject> {
                let path = format!("{}/manager", self.base_path());
                get_json(&self.t, &path, &QueryOptions::new()).await
            }

            /// The users who report to this user.
            pub async fn direct_reports(&self) -> Result<Vec<DirectoryObject>> {
                let path = format!("{}/directReports", self.base_path());
                pager(&self.t, &path, &QueryOptions::new())
                    .collect_all()
                    .await
            }

            /// The groups and directory roles this user is a direct member of.
            pub async fn member_of(&self) -> Result<Vec<DirectoryObject>> {
                let path = format!("{}/memberOf", self.base_path());
                pager(&self.t, &path, &QueryOptions::new())
                    .collect_all()
                    .await
            }

            /// Downloads the profile photo (JPEG bytes).
            pub async fn photo(&self) -> Result<Bytes> {
                let req = RawRequest::new(
                    Method::GET,
                    format!("{}/photo/$value", self.base_path()),
                );
                Ok(self.t.send(&req).await?.body)
            }

            /// Follows the sites with the given composite ids and returns the followed sites.
            pub async fn follow_sites<I, S>(&self, site_ids: I) -> Result<Vec<Site>>
            where
                I: IntoIterator<Item = S>,
                S: AsRef<str>,
            {
                self.followed_sites_action("add", site_ids).await
            }

            /// Stops following the sites with the given composite ids.
            pub async fn unfollow_sites<I, S>(&self, site_ids: I) -> Result<()>
            where
                I: IntoIterator<Item = S>,
                S: AsRef<str>,
            {
                self.followed_sites_action("remove", site_ids).await?;
                Ok(())
            }

            async fn followed_sites_action<I, S>(
                &self,
                action: &str,
                site_ids: I,
            ) -> Result<Vec<Site>>
            where
                I: IntoIterator<Item = S>,
                S: AsRef<str>,
            {
                let value: Vec<_> = site_ids
                    .into_iter()
                    .map(|id| json!({ "id": id.as_ref() }))
                    .collect();
                let mut req = RawRequest::new(
                    Method::POST,
                    format!("{}/followedSites/{action}", self.base_path()),
                );
                req.body = crate::http::Body::Json(json!({ "value": value }));
                let resp = self.t.send(&req).await?;
                if resp.body.is_empty() {
                    return Ok(Vec::new());
                }
                let doc: serde_json::Value = resp.json()?;
                Ok(match doc.get("value") {
                    Some(v) => serde_json::from_value(v.clone())?,
                    None => Vec::new(),
                })
            }
        }
    };
}

/// `/me` — the signed-in user.
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until a request is sent"]
pub struct MeRequest {
    t: Arc<Transport>,
    query: QueryOptions,
}

impl MeRequest {
    pub(crate) fn new(t: Arc<Transport>) -> Self {
        Self {
            t,
            query: QueryOptions::new(),
        }
    }

    /// Fetches the profile.
    pub async fn get(&self) -> Result<User> {
        get_json(&self.t, "/me", &self.query).await
    }

    /// The user's OneDrive.
    pub fn drive(&self) -> DriveRequest {
        DriveRequest::new(self.t.clone(), "/me/drive".to_owned())
    }
}
query_methods!(MeRequest);

impl MeRequest {
    fn base_path(&self) -> String {
        "/me".to_owned()
    }
}
user_relations!(MeRequest);

/// `/users` — the users collection.
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until a request is sent"]
pub struct UsersRequest {
    t: Arc<Transport>,
    query: QueryOptions,
}

impl UsersRequest {
    pub(crate) fn new(t: Arc<Transport>) -> Self {
        Self {
            t,
            query: QueryOptions::new(),
        }
    }

    /// Lists all users, following paging links.
    pub async fn list(&self) -> Result<Vec<User>> {
        self.pages().collect_all().await
    }

    /// Returns a pager to consume results page by page.
    pub fn pages(&self) -> Pager<User> {
        pager(&self.t, "/users", &self.query)
    }
}
query_methods!(UsersRequest);

/// `/users/{id}` — one user.
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until a request is sent"]
pub struct UserRequest {
    t: Arc<Transport>,
    path: String,
    query: QueryOptions,
}

impl UserRequest {
    pub(crate) fn new(t: Arc<Transport>, id: &str) -> Self {
        Self {
            t,
            path: format!("/users/{}", crate::http::encode_path(id)),
            query: QueryOptions::new(),
        }
    }

    /// Fetches the user.
    pub async fn get(&self) -> Result<User> {
        get_json(&self.t, &self.path, &self.query).await
    }
}
query_methods!(UserRequest);

impl UserRequest {
    fn base_path(&self) -> String {
        self.path.clone()
    }
}
user_relations!(UserRequest);

#[cfg(test)]
mod tests {
    use crate::graph::testutil::client;
    use serde_json::json;
    use wiremock::{
        Mock, ResponseTemplate,
        matchers::{header, method, path, query_param},
    };

    #[tokio::test]
    async fn relationships_photo_and_follow() {
        use wiremock::matchers::body_json;
        let (c, s) = client().await;
        Mock::given(method("GET"))
            .and(path("/me/manager"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"id":"m1","displayName":"Boss"})),
            )
            .mount(&s)
            .await;
        Mock::given(method("GET"))
            .and(path("/users/u1/directReports"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":[{"id":"r1"}]})))
            .mount(&s)
            .await;
        Mock::given(method("GET"))
            .and(path("/users/u1/memberOf"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":[{"id":"g1"}]})))
            .mount(&s)
            .await;
        Mock::given(method("GET"))
            .and(path("/me/photo/$value"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![1u8, 2, 3]))
            .mount(&s)
            .await;
        Mock::given(method("POST"))
            .and(path("/me/followedSites/add"))
            .and(body_json(json!({"value":[{"id":"s1"},{"id":"s2"}]})))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"value":[{"id":"s1"},{"id":"s2"}]})),
            )
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(method("POST"))
            .and(path("/users/u1/followedSites/remove"))
            .and(body_json(json!({"value":[{"id":"s1"}]})))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&s)
            .await;
        assert_eq!(
            c.me().manager().await.unwrap().display_name.as_deref(),
            Some("Boss")
        );
        let u = c.user("u1");
        assert_eq!(u.direct_reports().await.unwrap()[0].id, "r1");
        assert_eq!(u.member_of().await.unwrap()[0].id, "g1");
        assert_eq!(c.me().photo().await.unwrap().as_ref(), &[1, 2, 3]);
        assert_eq!(c.me().follow_sites(["s1", "s2"]).await.unwrap().len(), 2);
        u.unfollow_sites(["s1"]).await.unwrap();
    }

    #[tokio::test]
    async fn me_with_select() {
        let (c, s) = client().await;
        Mock::given(method("GET"))
            .and(path("/me"))
            .and(query_param("$select", "displayName,mail"))
            .and(header("authorization", "Bearer tok"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"id":"1","displayName":"Ann","mail":"a@x.com"})),
            )
            .mount(&s)
            .await;
        let me = c.me().select(["displayName", "mail"]).get().await.unwrap();
        assert_eq!(me.display_name.as_deref(), Some("Ann"));
    }

    #[tokio::test]
    async fn users_list_follows_pages() {
        let (c, s) = client().await;
        let next = format!("{}/users?page=2", s.uri());
        Mock::given(path("/users"))
            .and(query_param("page", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":[{"id":"3"}]})))
            .mount(&s)
            .await;
        Mock::given(path("/users"))
            .and(query_param("$top", "2"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"value":[{"id":"1"},{"id":"2"}],"@odata.nextLink":next})),
            )
            .mount(&s)
            .await;
        let users = c.users().top(2).list().await.unwrap();
        assert_eq!(
            users.iter().map(|u| u.id.as_str()).collect::<Vec<_>>(),
            ["1", "2", "3"]
        );
    }

    #[tokio::test]
    async fn user_by_upn_is_encoded() {
        let (c, s) = client().await;
        Mock::given(path("/users/a%40x.com"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":"9"})))
            .mount(&s)
            .await;
        assert_eq!(c.user("a@x.com").get().await.unwrap().id, "9");
    }
}
