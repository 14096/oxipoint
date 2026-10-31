//! The SharePoint web (`/web`) and everything addressed beneath it.

use std::sync::Arc;

use reqwest::Method;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use url::Url;

use super::{FolderInfo, Inner, NewWeb, RegionalSettings, SiteInfo, SpUser, Web, odata_str};
use crate::{
    Result,
    http::{Body, Pager, QueryOptions, RawRequest, encode_path, query::query_methods},
};

mod admin;
mod navigation;
mod principals;
mod schema;

pub use admin::{FeaturesRequest, RecycleBinItemRequest, RecycleBinRequest};
pub use navigation::{NavigationNodeRequest, NavigationNodesRequest, NavigationRequest};
pub use principals::{
    PermissionsRequest, RoleDefinitionsRequest, SiteGroupRequest, SiteGroupsRequest, UserRequest,
    UsersRequest,
};
pub use schema::{ContentTypeRequest, ContentTypesRequest, FieldRequest, FieldsRequest};

pub(crate) fn get_req(path: &str, q: &QueryOptions) -> RawRequest {
    let mut r = RawRequest::new(Method::GET, path);
    r.query = q.pairs().to_vec();
    r
}

pub(crate) fn json_body<F: Serialize>(f: &F) -> Result<Body> {
    Ok(Body::Json(serde_json::to_value(f)?))
}

/// POSTs with the form digest and deserializes the created entity.
pub(crate) async fn post<T: DeserializeOwned>(
    inner: &Inner,
    path: String,
    body: Body,
) -> Result<T> {
    let mut req = RawRequest::new(Method::POST, path);
    req.body = body;
    inner.write(req).await?.json()
}

/// POSTs with the form digest and discards the result.
pub(crate) async fn post_action(inner: &Inner, path: String, body: Body) -> Result<()> {
    let mut req = RawRequest::new(Method::POST, path);
    req.body = body;
    inner.write(req).await?;
    Ok(())
}

/// `/web` — the site itself, or a subweb.
#[derive(Debug, Clone)]
pub struct WebRequest {
    inner: Arc<Inner>,
    base: String,
    query: QueryOptions,
}

impl WebRequest {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self::at(inner, "/web".to_owned())
    }

    fn at(inner: Arc<Inner>, base: String) -> Self {
        Self {
            inner,
            base,
            query: QueryOptions::new(),
        }
    }

    async fn get_at<T: DeserializeOwned>(&self, suffix: &str) -> Result<T> {
        let req = get_req(&format!("{}{suffix}", self.base), &self.query);
        self.inner.t.send(&req).await?.json()
    }

    fn pager_at<T: DeserializeOwned>(&self, suffix: &str) -> Pager<T> {
        Pager::new(
            self.inner.t.clone(),
            get_req(&format!("{}{suffix}", self.base), &self.query),
        )
    }

    /// Fetches the web properties.
    pub async fn get(&self) -> Result<Web> {
        self.get_at("").await
    }

    /// Updates the given properties of the web (MERGE).
    pub async fn update<F: Serialize>(&self, properties: &F) -> Result<()> {
        self.inner
            .tunnel("MERGE", self.base.clone(), json_body(properties)?)
            .await?;
        Ok(())
    }

    /// Deletes the web.
    pub async fn delete(&self) -> Result<()> {
        self.inner
            .tunnel("DELETE", self.base.clone(), Body::None)
            .await?;
        Ok(())
    }

    /// Lists child webs.
    pub async fn subwebs(&self) -> Result<Vec<Web>> {
        self.pager_at("/webs").collect_all().await
    }

    /// A subweb by server-relative URL such as `/sites/team/child`.
    ///
    /// The returned handle talks to that web's own `/_api` endpoint, so every
    /// sub-handle (navigation, fields, ...) applies to it.
    pub fn subweb(&self, server_relative_url: &str) -> WebRequest {
        let origin = Url::parse(self.inner.t.base_url())
            .map(|u| u.origin().ascii_serialization())
            .unwrap_or_default();
        let path = encode_path(server_relative_url.trim_matches('/'));
        Self::at(self.inner.clone(), format!("{origin}/{path}/_api/web"))
    }

    /// Creates a subweb.
    pub async fn create_subweb(&self, new: &NewWeb) -> Result<Web> {
        post(
            &self.inner,
            format!("{}/webs/add", self.base),
            Body::Json(json!({ "parameters": new })),
        )
        .await
    }

    /// The parent web.
    pub async fn parent_web(&self) -> Result<Web> {
        self.get_at("/parentweb").await
    }

    /// The site collection this web belongs to.
    pub async fn site_info(&self) -> Result<SiteInfo> {
        let root = self.base.strip_suffix("/web").unwrap_or_default();
        self.inner
            .t
            .send(&get_req(&format!("{root}/site"), &self.query))
            .await?
            .json()
    }

    /// The root folder of the web.
    pub async fn root_folder(&self) -> Result<FolderInfo> {
        self.get_at("/rootfolder").await
    }

    /// The list templates of the web.
    pub async fn list_templates(&self) -> Result<Vec<Value>> {
        self.pager_at("/listtemplates").collect_all().await
    }

    /// The web templates available for a locale id.
    pub async fn web_templates(&self, lcid: u32) -> Result<Vec<Value>> {
        self.pager_at(&format!("/getavailablewebtemplates({lcid})"))
            .collect_all()
            .await
    }

    /// The signed-in user.
    pub async fn current_user(&self) -> Result<SpUser> {
        self.get_at("/currentuser").await
    }

    /// Resolves a login name to a user, adding it to the site when missing.
    pub async fn ensure_user(&self, login_name: &str) -> Result<SpUser> {
        post(
            &self.inner,
            format!("{}/ensureuser", self.base),
            Body::Json(json!({ "logonName": login_name })),
        )
        .await
    }

    /// The regional settings (use `expand("TimeZone")` on the web handle to include the time zone).
    pub async fn regional_settings(&self) -> Result<RegionalSettings> {
        self.get_at("/regionalsettings").await
    }

    /// Navigation nodes.
    pub fn navigation(&self) -> NavigationRequest {
        NavigationRequest::new(self.inner.clone(), format!("{}/navigation", self.base))
    }

    /// Site columns of the web.
    pub fn fields(&self) -> FieldsRequest {
        FieldsRequest::new(self.inner.clone(), format!("{}/fields", self.base))
    }

    /// Content types of the web.
    pub fn content_types(&self) -> ContentTypesRequest {
        ContentTypesRequest::new(self.inner.clone(), format!("{}/contenttypes", self.base))
    }

    /// Users of the site collection.
    pub fn users(&self) -> UsersRequest {
        UsersRequest::new(self.inner.clone(), format!("{}/siteusers", self.base))
    }

    /// Site groups.
    pub fn groups(&self) -> SiteGroupsRequest {
        SiteGroupsRequest::new(self.inner.clone(), format!("{}/sitegroups", self.base))
    }

    /// Role definitions.
    pub fn role_definitions(&self) -> RoleDefinitionsRequest {
        RoleDefinitionsRequest::new(self.inner.clone(), format!("{}/roledefinitions", self.base))
    }

    /// Role assignments and inheritance.
    pub fn permissions(&self) -> PermissionsRequest {
        PermissionsRequest::new(self.inner.clone(), self.base.clone())
    }

    /// Activated features.
    pub fn features(&self) -> FeaturesRequest {
        FeaturesRequest::new(self.inner.clone(), format!("{}/features", self.base))
    }

    /// The recycle bin.
    pub fn recycle_bin(&self) -> RecycleBinRequest {
        RecycleBinRequest::new(self.inner.clone(), format!("{}/recyclebin", self.base))
    }
}
query_methods!(WebRequest);

pub(crate) fn id_literal(id: &str) -> String {
    format!("'{}'", odata_str(id))
}

#[cfg(test)]
mod tests {
    use crate::sharepoint::{
        NewWeb,
        testutil::{client, client_retrying, mock_get, mock_write, mount_digest},
    };
    use serde_json::json;
    use wiremock::{
        Mock, ResponseTemplate,
        matchers::{header, method, path, query_param},
    };

    #[tokio::test]
    async fn get_web_and_subwebs() {
        let (c, s) = client().await;
        Mock::given(path("/_api/web"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"Id":"g","Title":"Team","Url":"https://x/sites/team"})),
            )
            .mount(&s)
            .await;
        Mock::given(path("/_api/web/webs"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"value":[{"Id":"c","Title":"Child"}]})),
            )
            .mount(&s)
            .await;
        let w = c.web().get().await.unwrap();
        assert_eq!(
            (w.title.as_deref(), w.url.as_deref()),
            (Some("Team"), Some("https://x/sites/team"))
        );
        assert_eq!(
            c.web().subwebs().await.unwrap()[0].title.as_deref(),
            Some("Child")
        );
    }

    #[tokio::test]
    async fn subwebs_follow_paging() {
        let (c, s) = client().await;
        let next = format!("{}/_api/web/webs?page=2", s.uri());
        Mock::given(path("/_api/web/webs"))
            .and(query_param("page", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":[{"Id":"b"}]})))
            .mount(&s)
            .await;
        Mock::given(path("/_api/web/webs"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"value":[{"Id":"a"}],"odata.nextLink":next})),
            )
            .mount(&s)
            .await;
        assert_eq!(c.web().subwebs().await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn update_and_delete_web() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        mock_write(
            &s,
            "/_api/web",
            Some("MERGE"),
            Some(json!({"Title":"New"})),
            json!(null),
        )
        .await;
        mock_write(&s, "/_api/web", Some("DELETE"), None, json!(null)).await;
        c.web().update(&json!({"Title":"New"})).await.unwrap();
        c.web().delete().await.unwrap();
    }

    #[tokio::test]
    async fn subweb_by_url_uses_its_own_api_and_sub_handles() {
        let (c, s) = client().await;
        mock_get(
            &s,
            "/sites/team/child/_api/web",
            json!({"Id":"c","Title":"Child"}),
        )
        .await;
        mock_get(
            &s,
            "/sites/team/child/_api/web/navigation/quicklaunch",
            json!({"value":[]}),
        )
        .await;
        let sub = c.web().subweb("/sites/team/child/");
        assert_eq!(sub.get().await.unwrap().title.as_deref(), Some("Child"));
        sub.navigation().quick_launch().list().await.unwrap();
    }

    #[tokio::test]
    async fn create_subweb_posts_parameters() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        mock_write(
            &s,
            "/_api/web/webs/add",
            None,
            Some(json!({"parameters":{"Title":"T","Url":"t","WebTemplate":"STS#3","UseSamePermissionsAsParentSite":true}})),
            json!({"Id":"n","Title":"T"}),
        )
        .await;
        let w = c
            .web()
            .create_subweb(&NewWeb::new("T", "t", "STS#3"))
            .await
            .unwrap();
        assert_eq!(w.id, "n");
    }

    #[tokio::test]
    async fn information_reads_hit_expected_paths() {
        let (c, s) = client().await;
        mock_get(&s, "/_api/web/parentweb", json!({"Id":"p"})).await;
        mock_get(&s, "/_api/site", json!({"Id":"sc","Url":"u"})).await;
        mock_get(
            &s,
            "/_api/web/rootfolder",
            json!({"Name":"r","ServerRelativeUrl":"/"}),
        )
        .await;
        mock_get(
            &s,
            "/_api/web/listtemplates",
            json!({"value":[{"Name":"Tasks"}]}),
        )
        .await;
        mock_get(
            &s,
            "/_api/web/getavailablewebtemplates(1033)",
            json!({"value":[{"Name":"STS#3"}]}),
        )
        .await;
        assert_eq!(c.web().parent_web().await.unwrap().id, "p");
        assert_eq!(c.web().site_info().await.unwrap().id, "sc");
        assert_eq!(c.web().root_folder().await.unwrap().name, "r");
        assert_eq!(c.web().list_templates().await.unwrap().len(), 1);
        assert_eq!(c.web().web_templates(1033).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn site_info_of_subweb_uses_its_own_site() {
        let (c, s) = client().await;
        mock_get(&s, "/sites/team/child/_api/site", json!({"Id":"sc"})).await;
        assert_eq!(
            c.web()
                .subweb("/sites/team/child")
                .site_info()
                .await
                .unwrap()
                .id,
            "sc"
        );
    }

    #[tokio::test]
    async fn users_and_ensure_user() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        mock_get(&s, "/_api/web/currentuser", json!({"Id":5})).await;
        mock_get(
            &s,
            "/_api/web/siteusers",
            json!({"value":[{"Id":5},{"Id":6}]}),
        )
        .await;
        mock_get(
            &s,
            "/_api/web/siteusers/getbyid(6)",
            json!({"Id":6,"Title":"B"}),
        )
        .await;
        mock_write(
            &s,
            "/_api/web/ensureuser",
            None,
            Some(json!({"logonName":"i:0#.f|membership|a@b.com"})),
            json!({"Id":9,"LoginName":"i:0#.f|membership|a@b.com"}),
        )
        .await;
        assert_eq!(c.web().current_user().await.unwrap().id, 5);
        assert_eq!(c.web().users().list().await.unwrap().len(), 2);
        assert_eq!(
            c.web()
                .users()
                .by_id(6)
                .get()
                .await
                .unwrap()
                .title
                .as_deref(),
            Some("B")
        );
        assert_eq!(
            c.web()
                .ensure_user("i:0#.f|membership|a@b.com")
                .await
                .unwrap()
                .id,
            9
        );
    }

    #[tokio::test]
    async fn groups_members_and_role_definitions() {
        let (c, s) = client().await;
        mock_get(
            &s,
            "/_api/web/sitegroups",
            json!({"value":[{"Id":5,"Title":"Owners"}]}),
        )
        .await;
        mock_get(
            &s,
            "/_api/web/sitegroups(5)",
            json!({"Id":5,"Title":"Owners"}),
        )
        .await;
        mock_get(
            &s,
            "/_api/web/sitegroups(5)/users",
            json!({"value":[{"Id":1}]}),
        )
        .await;
        mock_get(
            &s,
            "/_api/web/roledefinitions",
            json!({"value":[{"Id":1073741829,"Name":"Full Control"}]}),
        )
        .await;
        assert_eq!(c.web().groups().list().await.unwrap()[0].id, 5);
        let g = c.web().groups().by_id(5);
        assert_eq!(g.get().await.unwrap().title.as_deref(), Some("Owners"));
        assert_eq!(g.users().list().await.unwrap().len(), 1);
        assert_eq!(
            c.web().role_definitions().list().await.unwrap()[0]
                .name
                .as_deref(),
            Some("Full Control")
        );
    }

    #[tokio::test]
    async fn effective_permissions_and_role_assignments() {
        let (c, s) = client().await;
        mock_get(
            &s,
            "/_api/web/effectivebasepermissions",
            json!({"High":"432","Low":"1011028719"}),
        )
        .await;
        mock_get(
            &s,
            "/_api/web/roleassignments",
            json!({"value":[{"PrincipalId":5,"RoleDefinitionBindings":[{"Id":1,"Name":"Read"}]}]}),
        )
        .await;
        let p = c.web().permissions();
        let eff = p.effective().await.unwrap();
        assert_eq!((eff.high.as_str(), eff.low.as_str()), ("432", "1011028719"));
        let ra = p.role_assignments().await.unwrap();
        assert_eq!(
            ra[0].role_definition_bindings[0].name.as_deref(),
            Some("Read")
        );
    }

    #[tokio::test]
    async fn role_assignment_changes_and_inheritance() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        let p = c.web().permissions();
        for tail in [
            "roleassignments/addroleassignment(principalid=5,roledefid=1073741826)",
            "roleassignments/removeroleassignment(principalid=5,roledefid=1073741826)",
            "breakroleinheritance(copyRoleAssignments=true,clearSubscopes=false)",
            "resetroleinheritance",
        ] {
            mock_write(&s, &format!("/_api/web/{tail}"), None, None, json!(null)).await;
        }
        p.add_role_assignment(5, 1073741826).await.unwrap();
        p.remove_role_assignment(5, 1073741826).await.unwrap();
        p.break_inheritance(true, false).await.unwrap();
        p.reset_inheritance().await.unwrap();
    }

    #[tokio::test]
    async fn features_and_regional_settings() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        mock_get(
            &s,
            "/_api/web/features",
            json!({"value":[{"DefinitionId":"f1"}]}),
        )
        .await;
        mock_get(&s, "/_api/web/regionalsettings", json!({"LocaleId":1033})).await;
        mock_write(
            &s,
            "/_api/web/features/add",
            None,
            Some(json!({"featureId":"f1","force":false})),
            json!(null),
        )
        .await;
        mock_write(
            &s,
            "/_api/web/features/remove",
            None,
            Some(json!({"featureId":"f1","force":false})),
            json!(null),
        )
        .await;
        assert_eq!(
            c.web().features().list().await.unwrap()[0].definition_id,
            "f1"
        );
        c.web().features().activate("f1").await.unwrap();
        c.web().features().deactivate("f1").await.unwrap();
        assert_eq!(
            c.web().regional_settings().await.unwrap().locale_id,
            Some(1033)
        );
    }

    #[tokio::test]
    async fn recycle_bin_list_pages_and_actions() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        let next = format!("{}/_api/web/recyclebin?p=2", s.uri());
        Mock::given(path("/_api/web/recyclebin"))
            .and(query_param("p", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":[{"Id":"b"}]})))
            .mount(&s)
            .await;
        Mock::given(path("/_api/web/recyclebin"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"value":[{"Id":"a"}],"odata.nextLink":next})),
            )
            .mount(&s)
            .await;
        mock_write(
            &s,
            "/_api/web/recyclebin('a')/restore",
            None,
            None,
            json!(null),
        )
        .await;
        mock_write(
            &s,
            "/_api/web/recyclebin('a')/deleteobject",
            None,
            None,
            json!(null),
        )
        .await;
        mock_write(
            &s,
            "/_api/web/recyclebin/deleteall",
            None,
            None,
            json!(null),
        )
        .await;
        let bin = c.web().recycle_bin();
        assert_eq!(bin.list().await.unwrap().len(), 2);
        bin.item("a").restore().await.unwrap();
        bin.item("a").delete().await.unwrap();
        bin.empty().await.unwrap();
    }

    #[tokio::test]
    async fn throttled_write_is_retried_with_same_digest() {
        let (c, s) = client_retrying().await;
        mount_digest(&s, 1).await;
        Mock::given(method("POST"))
            .and(path("/_api/web"))
            .and(header("x-requestdigest", "DIGEST"))
            .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "0"))
            .up_to_n_times(1)
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(method("POST"))
            .and(path("/_api/web"))
            .and(header("x-requestdigest", "DIGEST"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&s)
            .await;
        c.web().update(&json!({"Title":"x"})).await.unwrap();
    }
}
