use std::sync::Arc;

use super::{DriveRequest, User, get_json, pager};
use crate::{
    Result,
    http::{Pager, QueryOptions, Transport, query::query_methods},
};

/// `/me` — the signed-in user.
#[derive(Debug, Clone)]
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

/// `/users` — the users collection.
#[derive(Debug, Clone)]
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

#[cfg(test)]
mod tests {
    use crate::graph::testutil::client;
    use serde_json::json;
    use wiremock::{
        Mock, ResponseTemplate,
        matchers::{header, method, path, query_param},
    };

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
