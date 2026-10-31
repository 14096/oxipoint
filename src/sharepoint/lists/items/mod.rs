mod attachments;
mod comments;

pub use attachments::{AttachmentRequest, AttachmentsRequest};
pub use comments::CommentsRequest;

use std::sync::Arc;

use reqwest::Method;
use serde::Serialize;

use serde_json::{Map, Value, json};

use crate::{
    Error, Result,
    fields::Fields,
    http::{Body, Pager, QueryOptions, RawRequest, query::query_methods},
    sharepoint::{
        ContentType, FieldResult, FileInfo, FolderInfo, Inner, ItemVersion, ListInfo, ListItem,
        PermissionsRequest, SpUser, ValidateOptions,
        inner::{get_req, json_body, post, post_action},
    },
};

fn validate_body(fields: &Fields, opts: &ValidateOptions) -> Result<Map<String, Value>> {
    let form: Vec<Value> = fields
        .form_values()?
        .into_iter()
        .map(|(n, v)| json!({ "FieldName": n, "FieldValue": v }))
        .collect();
    let mut body = Map::new();
    body.insert("formValues".into(), Value::Array(form));
    body.insert("bNewDocumentUpdate".into(), opts.new_document_update.into());
    body.insert(
        "checkInComment".into(),
        opts.check_in_comment
            .clone()
            .map_or(Value::Null, Value::String),
    );
    Ok(body)
}

fn field_results(doc: Value) -> Result<Vec<FieldResult>> {
    let items = doc
        .get("value")
        .or_else(|| doc.pointer("/d/results"))
        .cloned()
        .ok_or_else(|| Error::Config("validated write returned no results".into()))?;
    Ok(serde_json::from_value(items)?)
}

/// A list's items collection.
#[derive(Debug, Clone)]
pub struct ItemsRequest {
    inner: Arc<Inner>,
    base: String,
    list_base: String,
    query: QueryOptions,
}

impl ItemsRequest {
    pub(crate) fn new(inner: Arc<Inner>, base: String, list_base: String) -> Self {
        Self {
            inner,
            base,
            list_base,
            query: QueryOptions::new(),
        }
    }

    /// Lists items, following paging links.
    pub async fn list(&self) -> Result<Vec<ListItem>> {
        self.pages().collect_all().await
    }

    /// Returns a pager.
    pub fn pages(&self) -> Pager<ListItem> {
        Pager::new(self.inner.t.clone(), get_req(&self.base, &self.query))
    }

    /// Creates an item from column values.
    pub async fn create<F: Serialize>(&self, fields: &F) -> Result<ListItem> {
        let mut req = RawRequest::new(Method::POST, self.base.as_str());
        req.body = Body::Json(serde_json::to_value(fields)?);
        self.inner.write(req).await?.json()
    }

    /// Creates an item with form validation, optionally inside a folder.
    ///
    /// `folder_path` is the server-relative URL of the folder, such as
    /// `/sites/t/Lists/Tasks/Open`. Columns are sent as formatted text, so only kinds with a
    /// text form are accepted (not people, multi-lookups or multi-term metadata). The result
    /// has one entry per column; a rejected column is not an error of the call.
    pub async fn validate_create(
        &self,
        fields: &Fields,
        folder_path: Option<&str>,
        opts: &ValidateOptions,
    ) -> Result<Vec<FieldResult>> {
        let mut body = validate_body(fields, opts)?;
        let mut info = json!({ "UnderlyingObjectType": 0 });
        if let (Some(p), Some(o)) = (folder_path, info.as_object_mut()) {
            o.insert(
                "FolderPath".into(),
                json!({"__metadata": {"type": "SP.ResourcePath"}, "DecodedUrl": p}),
            );
        }
        body.insert("listItemCreateInfo".into(), info);
        let doc: Value = post(
            &self.inner,
            format!("{}/AddValidateUpdateItemUsingPath", self.list_base),
            Body::Json(Value::Object(body)),
        )
        .await?;
        field_results(doc)
    }

    /// Queries items with a CAML `<View>` document (`GetItems`), following paging links.
    pub async fn caml(&self, view_xml: &str) -> Result<Pager<ListItem>> {
        let digest = self.inner.digest().await?;
        let mut req = RawRequest::new(Method::POST, format!("{}/getitems", self.list_base));
        req.headers.push(("X-RequestDigest".into(), digest));
        req.body = Body::Json(json!({
            "query": {"__metadata": {"type": "SP.CamlQuery"}, "ViewXml": view_xml}
        }));
        Ok(Pager::new(self.inner.t.clone(), req))
    }

    /// Counts the items (honours `filter`).
    pub async fn count(&self) -> Result<u64> {
        let resp = self
            .inner
            .t
            .send(&get_req(&format!("{}/$count", self.base), &self.query))
            .await?;
        let text = resp.text();
        let text = text.trim();
        if let Ok(n) = text.parse() {
            return Ok(n);
        }
        let v: serde_json::Value = serde_json::from_str(text)?;
        let n = v.get("value").unwrap_or(&v);
        n.as_u64()
            .or_else(|| n.as_str().and_then(|s| s.parse().ok()))
            .ok_or_else(|| crate::Error::Config(format!("unexpected count response: {text}")))
    }
}
query_methods!(ItemsRequest);

/// One list item.
#[derive(Debug, Clone)]
pub struct ItemRequest {
    inner: Arc<Inner>,
    base: String,
    query: QueryOptions,
}

impl ItemRequest {
    pub(crate) fn new(inner: Arc<Inner>, base: String) -> Self {
        Self {
            inner,
            base,
            query: QueryOptions::new(),
        }
    }

    /// Fetches the item.
    pub async fn get(&self) -> Result<ListItem> {
        self.inner
            .t
            .send(&get_req(&self.base, &self.query))
            .await?
            .json()
    }

    /// Updates the given columns (MERGE).
    pub async fn update<F: Serialize>(&self, fields: &F) -> Result<()> {
        self.inner
            .tunnel("MERGE", self.base.clone(), json_body(fields)?)
            .await?;
        Ok(())
    }

    /// Updates the given columns only if the item still has `etag` (sent as `IF-MATCH`).
    pub async fn update_with_etag<F: Serialize>(&self, fields: &F, etag: &str) -> Result<()> {
        self.inner
            .tunnel_match("MERGE", self.base.clone(), json_body(fields)?, etag)
            .await?;
        Ok(())
    }

    /// Updates columns with form validation and reports a result per column.
    ///
    /// Columns are sent as formatted text, so only kinds with a text form are accepted.
    pub async fn validate_update(
        &self,
        fields: &Fields,
        opts: &ValidateOptions,
    ) -> Result<Vec<FieldResult>> {
        let body = validate_body(fields, opts)?;
        let doc: Value = post(
            &self.inner,
            format!("{}/ValidateUpdateListItem", self.base),
            Body::Json(Value::Object(body)),
        )
        .await?;
        field_results(doc)
    }

    async fn read<T: serde::de::DeserializeOwned>(&self, suffix: &str) -> Result<T> {
        self.inner
            .t
            .send(&get_req(&format!("{}/{suffix}", self.base), &self.query))
            .await?
            .json()
    }

    /// The values as the text shown in views.
    pub async fn field_values_as_text(&self) -> Result<Map<String, Value>> {
        self.read("FieldValuesAsText").await
    }

    /// The values as HTML.
    pub async fn field_values_as_html(&self) -> Result<Map<String, Value>> {
        self.read("FieldValuesAsHtml").await
    }

    /// The values as shown in the edit form.
    pub async fn field_values_for_edit(&self) -> Result<Map<String, Value>> {
        self.read("FieldValuesForEdit").await
    }

    /// The item's content type.
    pub async fn content_type(&self) -> Result<ContentType> {
        self.read("ContentType").await
    }

    /// The list the item belongs to.
    pub async fn parent_list(&self) -> Result<ListInfo> {
        self.read("ParentList").await
    }

    /// The file of a document item.
    pub async fn file(&self) -> Result<FileInfo> {
        self.read("File").await
    }

    /// The folder of a folder item.
    pub async fn folder(&self) -> Result<FolderInfo> {
        self.read("Folder").await
    }

    /// Likes the item as the signed-in user.
    pub async fn like(&self) -> Result<()> {
        post_action(&self.inner, format!("{}/like", self.base), Body::None).await
    }

    /// Removes the signed-in user's like.
    pub async fn unlike(&self) -> Result<()> {
        post_action(&self.inner, format!("{}/unlike", self.base), Body::None).await
    }

    /// The users who liked the item.
    pub async fn liked_by(&self) -> Result<Vec<SpUser>> {
        let mut req = get_req(&format!("{}/likedByInformation", self.base), &self.query);
        req.query.push(("$expand".into(), "likedBy".into()));
        let doc: Value = self.inner.t.send(&req).await?.json()?;
        let users = doc
            .get("likedBy")
            .or_else(|| doc.get("LikedBy"))
            .cloned()
            .unwrap_or(Value::Array(vec![]));
        Ok(serde_json::from_value(users)?)
    }

    /// The item's comments.
    pub fn comments(&self) -> CommentsRequest {
        CommentsRequest::new(self.inner.clone(), format!("{}/Comments", self.base))
    }

    /// Restores the version with the given label (such as `2.0`).
    pub async fn restore_version(&self, label: &str) -> Result<()> {
        post_action(
            &self.inner,
            format!("{}/Versions/restoreByLabel", self.base),
            Body::Json(json!({ "versionLabel": label })),
        )
        .await
    }

    /// Deletes the item permanently.
    pub async fn delete(&self) -> Result<()> {
        self.inner
            .tunnel("DELETE", self.base.clone(), Body::None)
            .await?;
        Ok(())
    }

    /// Moves the item to the recycle bin.
    pub async fn recycle(&self) -> Result<()> {
        post_action(&self.inner, format!("{}/recycle", self.base), Body::None).await
    }

    /// The versions of the item.
    pub async fn versions(&self) -> Result<Vec<ItemVersion>> {
        Pager::new(
            self.inner.t.clone(),
            get_req(&format!("{}/versions", self.base), &self.query),
        )
        .collect_all()
        .await
    }

    /// The item's attachments.
    pub fn attachments(&self) -> AttachmentsRequest {
        AttachmentsRequest::new(self.inner.clone(), format!("{}/AttachmentFiles", self.base))
    }

    /// Role assignments and inheritance of the item.
    pub fn permissions(&self) -> PermissionsRequest {
        PermissionsRequest::new(self.inner.clone(), self.base.clone())
    }
}
query_methods!(ItemRequest);

#[cfg(test)]
mod tests {
    use crate::{
        Error,
        sharepoint::{
            ChangeQuery,
            testutil::{client, mock_get, mock_write, mount_digest},
        },
    };
    use serde_json::json;
    use wiremock::{
        Mock, ResponseTemplate,
        matchers::{body_bytes, body_json, header, method, path, query_param},
    };

    #[tokio::test]
    async fn filtered_query_is_encoded() {
        let (c, s) = client().await;
        Mock::given(path("/_api/web/lists/getbytitle('T')/items"))
            .and(query_param("$filter", "Status eq 'Open'"))
            .and(query_param("$select", "Id,Title"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"value":[{"Id":1,"Title":"x"}]})),
            )
            .mount(&s)
            .await;
        let items = c
            .list_by_title("T")
            .items()
            .filter("Status eq 'Open'")
            .select(["Id", "Title"])
            .list()
            .await
            .unwrap();
        assert_eq!(
            (items[0].id, items[0].fields["Title"].clone()),
            (1, json!("x"))
        );
    }

    #[tokio::test]
    async fn update_uses_merge_headers() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        Mock::given(method("POST"))
            .and(path("/_api/web/lists/getbytitle('T')/items(4)"))
            .and(header("x-http-method", "MERGE"))
            .and(header("if-match", "*"))
            .and(header("x-requestdigest", "DIGEST"))
            .and(body_json(json!({"Title":"new"})))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&s)
            .await;
        c.list_by_title("T")
            .item(4)
            .update(&json!({"Title":"new"}))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn delete_uses_tunnel_and_reports_errors() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        Mock::given(method("POST"))
            .and(path("/_api/web/lists/getbytitle('T')/items(9)"))
            .and(header("x-http-method", "DELETE"))
            .respond_with(ResponseTemplate::new(404).set_body_json(json!({"odata.error":{"code":"-2147024809","message":{"lang":"en","value":"missing"}}})))
            .mount(&s)
            .await;
        let e = c.list_by_title("T").item(9).delete().await.unwrap_err();
        assert!(matches!(e, Error::Http { status: 404, ref message, .. } if message == "missing"));
    }

    #[tokio::test]
    async fn update_with_etag_overrides_if_match() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        Mock::given(method("POST"))
            .and(path("/_api/web/lists/getbytitle('T')/items(4)"))
            .and(header("x-http-method", "MERGE"))
            .and(header("if-match", "\"3\""))
            .and(header("x-requestdigest", "DIGEST"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&s)
            .await;
        c.list_by_title("T")
            .item(4)
            .update_with_etag(&json!({"Title":"n"}), "\"3\"")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn recycle_and_versions() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        let p = "/_api/web/lists/getbytitle('T')/items(4)";
        mock_write(&s, &format!("{p}/recycle"), None, None, json!(null)).await;
        mock_get(
            &s,
            &format!("{p}/versions"),
            json!({"value":[{"VersionLabel":"2.0"},{"VersionLabel":"1.0"}]}),
        )
        .await;
        let item = c.list_by_title("T").item(4);
        item.recycle().await.unwrap();
        assert_eq!(item.versions().await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn caml_query_posts_view_and_pages() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        let next = format!("{}/_api/web/lists/getbytitle('T')/items?page=2", s.uri());
        Mock::given(method("POST"))
            .and(path("/_api/web/lists/getbytitle('T')/getitems"))
            .and(header("x-requestdigest", "DIGEST"))
            .and(body_json(
                json!({"query":{"__metadata":{"type":"SP.CamlQuery"},"ViewXml":"<View/>"}}),
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"value":[{"Id":1}],"odata.nextLink":next})),
            )
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(method("GET"))
            .and(path("/_api/web/lists/getbytitle('T')/items"))
            .and(query_param("page", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":[{"Id":2}]})))
            .expect(1)
            .mount(&s)
            .await;
        let items = c
            .list_by_title("T")
            .items()
            .caml("<View/>")
            .await
            .unwrap()
            .collect_all()
            .await
            .unwrap();
        assert_eq!(items.iter().map(|i| i.id).collect::<Vec<_>>(), vec![1, 2]);
    }

    #[tokio::test]
    async fn count_reads_plain_and_json_values() {
        let (c, s) = client().await;
        Mock::given(path("/_api/web/lists/getbytitle('T')/items/$count"))
            .and(query_param("$filter", "A eq 1"))
            .respond_with(ResponseTemplate::new(200).set_body_string("42"))
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(path("/_api/web/lists/getbytitle('U')/items/$count"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":"7"})))
            .expect(1)
            .mount(&s)
            .await;
        assert_eq!(
            c.list_by_title("T")
                .items()
                .filter("A eq 1")
                .count()
                .await
                .unwrap(),
            42
        );
        assert_eq!(c.list_by_title("U").items().count().await.unwrap(), 7);
    }

    #[tokio::test]
    async fn get_changes_posts_query() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        let q = ChangeQuery::items().since("1;3;x;1;2");
        mock_write(
            &s,
            "/_api/web/lists/getbytitle('T')/getchanges",
            None,
            Some(q.to_body()),
            json!({"value":[{"ChangeType":2,"ChangeToken":{"StringValue":"1;3;x;5;9"},"ItemId":7}]}),
        )
        .await;
        let ch = c.list_by_title("T").get_changes(&q).await.unwrap();
        assert_eq!((ch[0].change_type, ch[0].item_id), (2, Some(7)));
        assert_eq!(ch[0].change_token, "1;3;x;5;9");
    }

    #[tokio::test]
    async fn attachments_list_add_delete() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        let p = "/_api/web/lists/getbytitle('T')/items(7)/AttachmentFiles";
        mock_get(&s, p, json!({"value":[{"FileName":"a.txt"}]})).await;
        Mock::given(method("POST"))
            .and(path(format!("{p}/add(FileName='a.txt')")))
            .and(header("x-requestdigest", "DIGEST"))
            .and(body_bytes(b"hello".to_vec()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"FileName":"a.txt"})))
            .expect(1)
            .mount(&s)
            .await;
        mock_write(
            &s,
            &format!("{p}('a.txt')"),
            Some("DELETE"),
            None,
            json!(null),
        )
        .await;
        let att = c.list_by_title("T").item(7).attachments();
        assert_eq!(att.list().await.unwrap()[0].file_name, "a.txt");
        assert_eq!(att.add("a.txt", "hello").await.unwrap().file_name, "a.txt");
        att.by_name("a.txt").delete().await.unwrap();
    }

    #[tokio::test]
    async fn item_permissions_break_inheritance_url() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        mock_write(
            &s,
            "/_api/web/lists/getbytitle('T')/items(7)/breakroleinheritance(copyRoleAssignments=false,clearSubscopes=false)",
            None,
            None,
            json!(null),
        )
        .await;
        c.list_by_title("T")
            .item(7)
            .permissions()
            .break_inheritance(false, false)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn list_permissions_use_list_path() {
        let (c, s) = client().await;
        mock_get(
            &s,
            "/_api/web/lists/getbytitle('T')/effectivebasepermissions",
            json!({"High":"0","Low":"1"}),
        )
        .await;
        mock_get(
            &s,
            "/_api/web/lists/getbytitle('T')/roleassignments",
            json!({"value":[{"PrincipalId":3}]}),
        )
        .await;
        let p = c.list_by_title("T").permissions();
        assert_eq!(p.effective().await.unwrap().low, "1");
        assert_eq!(p.role_assignments().await.unwrap()[0].principal_id, 3);
    }

    #[tokio::test]
    async fn typed_fields_are_accepted_as_item_body() {
        use crate::fields::Fields;
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        mock_write(
            &s,
            "/_api/web/lists/getbytitle('T')/items",
            None,
            Some(json!({"Title":"a","ProjectId":3})),
            json!({"Id":1,"Title":"a","ProjectId":3,"Done":true,"Due":"2030-01-02T03:04:05Z"}),
        )
        .await;
        let f = Fields::new().text("Title", "a").lookup("Project", 3);
        let item = c
            .list_by_title("T")
            .items()
            .create(&f.for_sharepoint())
            .await
            .unwrap();
        assert_eq!(item.lookup("Project").unwrap().id, 3);
        assert_eq!(item.bool("Done"), Some(true));
        assert_eq!(item.datetime("Due").unwrap().year, 2030);
        assert_eq!(item.text("Nope"), None);
    }

    #[tokio::test]
    async fn unsupported_kind_sends_nothing() {
        use crate::fields::{FieldValue, Fields};
        let (c, s) = client().await;
        let f = Fields::new().push("D", FieldValue::MultiTaxonomy(vec![]));
        let e = c
            .list_by_title("T")
            .items()
            .create(&f.for_sharepoint())
            .await;
        assert!(e.is_err());
        assert!(s.received_requests().await.unwrap().is_empty());
    }

    const ITEM: &str = "/_api/web/lists/getbytitle('T')/items(7)";

    #[tokio::test]
    async fn validate_update_returns_per_column_results() {
        use crate::{fields::Fields, sharepoint::ValidateOptions};
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        mock_write(
            &s,
            &format!("{ITEM}/ValidateUpdateListItem"),
            None,
            Some(json!({
                "formValues":[{"FieldName":"Due","FieldValue":"nope"},{"FieldName":"Title","FieldValue":"t"}],
                "bNewDocumentUpdate":false,"checkInComment":null
            })),
            json!({"value":[
                {"FieldName":"Title","FieldValue":"t","HasException":false},
                {"FieldName":"Due","HasException":true,"ErrorMessage":"Invalid date"}
            ]}),
        )
        .await;
        let f = Fields::new().datetime("Due", "nope").text("Title", "t");
        let r = c
            .list_by_title("T")
            .item(7)
            .validate_update(&f, &ValidateOptions::default())
            .await
            .unwrap();
        assert!(r[0].is_ok());
        assert_eq!(
            (r[1].is_ok(), r[1].error_message.as_deref()),
            (false, Some("Invalid date"))
        );
    }

    #[tokio::test]
    async fn validate_create_in_folder_path() {
        use crate::{fields::Fields, sharepoint::ValidateOptions};
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        mock_write(
            &s,
            "/_api/web/lists/getbytitle('T')/AddValidateUpdateItemUsingPath",
            None,
            Some(json!({
                "listItemCreateInfo":{
                    "UnderlyingObjectType":0,
                    "FolderPath":{"__metadata":{"type":"SP.ResourcePath"},"DecodedUrl":"/sites/t/Lists/Tasks/Open"}
                },
                "formValues":[{"FieldName":"Title","FieldValue":"t"}],
                "bNewDocumentUpdate":true,"checkInComment":"hi"
            })),
            json!({"value":[{"FieldName":"Title","HasException":false},{"FieldName":"Id","FieldValue":"12","HasException":false,"ItemId":12}]}),
        )
        .await;
        let opts = ValidateOptions {
            new_document_update: true,
            check_in_comment: Some("hi".into()),
        };
        let r = c
            .list_by_title("T")
            .items()
            .validate_create(
                &Fields::new().text("Title", "t"),
                Some("/sites/t/Lists/Tasks/Open"),
                &opts,
            )
            .await
            .unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[1].item_id.as_deref(), Some("12"));
    }

    #[tokio::test]
    async fn validate_rejects_kinds_without_text_form_before_sending() {
        use crate::{fields::Fields, sharepoint::ValidateOptions};
        let (c, s) = client().await;
        let r = c
            .list_by_title("T")
            .item(7)
            .validate_update(
                &Fields::new().person("Owner", 1),
                &ValidateOptions::default(),
            )
            .await;
        assert!(matches!(r, Err(crate::Error::Config(_))));
        assert!(s.received_requests().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn formatted_values_and_relations_hit_expected_paths() {
        let (c, s) = client().await;
        mock_get(
            &s,
            &format!("{ITEM}/FieldValuesAsText"),
            json!({"Title":"t","Due":"1/2/2030"}),
        )
        .await;
        mock_get(
            &s,
            &format!("{ITEM}/FieldValuesAsHtml"),
            json!({"Title":"<b>t</b>"}),
        )
        .await;
        mock_get(
            &s,
            &format!("{ITEM}/FieldValuesForEdit"),
            json!({"Title":"t"}),
        )
        .await;
        mock_get(
            &s,
            &format!("{ITEM}/ContentType"),
            json!({"Id":{"StringValue":"0x0100"},"Name":"Item"}),
        )
        .await;
        mock_get(
            &s,
            &format!("{ITEM}/ParentList"),
            json!({"Id":"l","Title":"T"}),
        )
        .await;
        mock_get(
            &s,
            &format!("{ITEM}/File"),
            json!({"Name":"a.docx","ServerRelativeUrl":"/x/a.docx"}),
        )
        .await;
        mock_get(
            &s,
            &format!("{ITEM}/Folder"),
            json!({"Name":"f","ServerRelativeUrl":"/x/f"}),
        )
        .await;
        let item = c.list_by_title("T").item(7);
        assert_eq!(
            item.field_values_as_text().await.unwrap()["Due"],
            "1/2/2030"
        );
        assert_eq!(
            item.field_values_as_html().await.unwrap()["Title"],
            "<b>t</b>"
        );
        assert_eq!(item.field_values_for_edit().await.unwrap().len(), 1);
        assert_eq!(item.content_type().await.unwrap().id, "0x0100");
        assert_eq!(item.parent_list().await.unwrap().id, "l");
        assert_eq!(item.file().await.unwrap().name, "a.docx");
        assert_eq!(item.folder().await.unwrap().name, "f");
    }

    #[tokio::test]
    async fn like_unlike_and_liked_by() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        mock_write(&s, &format!("{ITEM}/like"), None, None, json!(null)).await;
        mock_write(&s, &format!("{ITEM}/unlike"), None, None, json!(null)).await;
        Mock::given(method("GET"))
            .and(path(format!("{ITEM}/likedByInformation")))
            .and(query_param("$expand", "likedBy"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"likedBy":[{"Id":5,"Title":"A"}]})),
            )
            .expect(1)
            .mount(&s)
            .await;
        let item = c.list_by_title("T").item(7);
        item.like().await.unwrap();
        item.unlike().await.unwrap();
        assert_eq!(item.liked_by().await.unwrap()[0].id, 5);
    }

    #[tokio::test]
    async fn comments_list_add_delete() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        mock_get(
            &s,
            &format!("{ITEM}/Comments"),
            json!({"value":[{"id":"1","text":"hi"}]}),
        )
        .await;
        mock_write(
            &s,
            &format!("{ITEM}/Comments"),
            None,
            Some(json!({"text":"Done"})),
            json!({"id":"2","text":"Done"}),
        )
        .await;
        mock_write(
            &s,
            &format!("{ITEM}/Comments(2)"),
            Some("DELETE"),
            None,
            json!(null),
        )
        .await;
        let cm = c.list_by_title("T").item(7).comments();
        assert_eq!(cm.list().await.unwrap()[0].text.as_deref(), Some("hi"));
        let made = cm.add("Done").await.unwrap();
        assert_eq!(
            (made.id.as_str(), made.text.as_deref()),
            ("2", Some("Done"))
        );
        cm.delete("2").await.unwrap();
    }

    #[tokio::test]
    async fn restore_version_by_label() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        mock_write(
            &s,
            &format!("{ITEM}/Versions/restoreByLabel"),
            None,
            Some(json!({"versionLabel":"2.0"})),
            json!(null),
        )
        .await;
        c.list_by_title("T")
            .item(7)
            .restore_version("2.0")
            .await
            .unwrap();
    }
}
