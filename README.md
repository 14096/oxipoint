# oxipoint

> **Work in progress.**  
> This project is not production ready.

A fluent, type-safe Rust SDK for **SharePoint REST** and **Microsoft Graph**.

- Four sign-in methods: username/password, certificate, interactive browser, device code.
- Typed, chainable APIs for the main entities: sites and webs (navigation, users, groups,
  permissions, features, recycle bin), lists (fields, views, content types, webhooks), items
  (typed column values, validated writes, likes, comments, delta and change tracking), modern
  pages (Graph and SharePoint REST), files and folders (copy, move, versions, streaming), search,
  sharing links, batching and users.
- Typed column values (`oxipoint::fields`) that serialize for either SharePoint or Graph.
- A generic request builder for everything else, with the same authentication, retry and error handling.
- Async (tokio), rustls TLS, modular via cargo features, unit-tested against mocked servers.

```rust
use oxipoint::{auth::DeviceCodeCredential, graph::GraphClient};

#[tokio::main]
async fn main() -> oxipoint::Result<()> {
    let cred = DeviceCodeCredential::new("contoso.onmicrosoft.com", "<client-id>", |info| {
        println!("{}", info.message);
    });
    let graph = GraphClient::new(cred);

    let me = graph.me().select(["displayName", "mail"]).get().await?;
    println!("Hello {}", me.display_name.unwrap_or_default());
    Ok(())
}
```

## Authentication

Every sign-in method implements `auth::TokenCredential`, so any of them works with both
`GraphClient` and `SharePointClient`. Pick the method by who the program acts as:

| Flow                       | Acts as            | Type                    | Feature            |
| -------------------------- | ------------------ | ----------------------- | ------------------ |
| Interactive browser (PKCE) | the signed-in user | `BrowserCredential`     | `auth-browser`     |
| Device code                | the signed-in user | `DeviceCodeCredential`  | `auth-device-code` |
| Username/password          | a user, no prompt  | `PasswordCredential`    | `auth-password`    |
| Certificate                | the application    | `CertificateCredential` | `auth-certificate` |

Rules of thumb:

- **Tools used by people**: browser (desktop) or device code (SSH sessions, servers, no browser).
- **Unattended jobs and services**: certificate. This is also the only app-only method that
  SharePoint REST accepts (it rejects client secrets).
- **Username/password** is a legacy flow. It fails for accounts with MFA, federated sign-in or
  conditional access, and Microsoft discourages it. Use it only for service accounts in test tenants.

### Parameters

- **tenant**: the tenant ID (GUID) **or** a verified domain such as `contoso.onmicrosoft.com`.
  The short name `contoso` is not valid. Delegated flows also accept `organizations` (work or
  school accounts from any tenant) and `common`. Certificate auth needs a real tenant ID or domain.
- **client_id**: the Application (client) ID of your Entra app registration.

### Setting up the app registration (Entra ID, app registrations)

Common to all flows:

1. Create an app registration and note its **Application (client) ID** and **Directory (tenant) ID**.
2. Under **API permissions**, add what you need and grant admin consent where required:
   - Graph: for example `Sites.ReadWrite.All`, `Files.ReadWrite.All`, `User.Read`
     (_delegated_ for user flows, _application_ for certificate).
   - SharePoint REST: for example `AllSites.Read` / `AllSites.FullControl` (delegated) or
     `Sites.Selected` / `Sites.ReadWrite.All` (application) on _SharePoint_.

The SDK requests the `/.default` scope (`https://graph.microsoft.com/.default` and
`https://<tenant>.sharepoint.com/.default`), so it receives the permissions that are consented
on the app registration. If a call returns 403, the permission or the consent is missing there.

Per flow:

| Flow              | Authentication settings to configure                                                            |
| ----------------- | ----------------------------------------------------------------------------------------------- |
| Browser           | Add platform **Mobile and desktop** with redirect URI `http://localhost`. Enable public client. |
| Device code       | **Allow public client flows** = Yes.                                                            |
| Username/password | **Allow public client flows** = Yes; user must not require MFA.                                 |
| Certificate       | Upload the public certificate under **Certificates & secrets**. Use application permissions.    |

### Interactive browser

Opens the system browser, listens on `http://localhost:<port>` for the redirect, and exchanges
the code with PKCE. The `state` value is verified, and the wait times out after 5 minutes.

```rust
use std::time::Duration;
use oxipoint::auth::BrowserCredential;

let cred = BrowserCredential::new("contoso.onmicrosoft.com", "<client-id>")
    .with_port(8400)                     // optional: fixed port (default: random free port)
    .with_timeout(Duration::from_secs(120)); // optional
```

### Device code

Prints a code the user enters at `https://microsoft.com/devicelogin` on any device. The callback
receives `DeviceCodeInfo` (`user_code`, `verification_uri`, `message`, ...); show it however you like.

```rust
use oxipoint::auth::DeviceCodeCredential;

let cred = DeviceCodeCredential::new("contoso.onmicrosoft.com", "<client-id>", |info| {
    println!("{}", info.message);
});
```

### Username and password

```rust
use oxipoint::auth::PasswordCredential;

let cred = PasswordCredential::new(
    "contoso.onmicrosoft.com",
    "<client-id>",
    "user@contoso.com",
    std::env::var("APP_PASSWORD").unwrap(), // never hardcode passwords
);
```

### Certificate

Needs the certificate and its RSA private key, either as PEM or as a PFX (PKCS#12) file. Invalid
material is reported as `Error::Config` before any network call.

```rust
use oxipoint::auth::CertificateCredential;

// PEM certificate + PKCS#8/PKCS#1 private key
let cred = CertificateCredential::from_pem(
    "<tenant-id>", "<client-id>",
    &std::fs::read_to_string("cert.pem")?,
    &std::fs::read_to_string("key.pem")?,
)?;

// or a PFX archive
let cred = CertificateCredential::from_pfx(
    "<tenant-id>", "<client-id>", &std::fs::read("cert.pfx")?, "pfx-password",
)?;
```

### Token handling

- Clients wrap the credential in a cache: tokens are kept per scope and renewed about two minutes
  before they expire. Concurrent requests share one sign-in.
- Browser and device code credentials keep the refresh token and use it to get tokens for other
  scopes (for example Graph and then SharePoint) without prompting again. Use **one credential
  instance per client**: credentials other than `PasswordCredential` are not `Clone`, so a second
  client built from a second instance signs in again.
- Passwords, access tokens and refresh tokens are wrapped in `Secret` and print as `[REDACTED]` in
  `Debug` output. Errors never contain them.
- All credentials accept `.with_authority(url)` to target a national cloud, for example
  `https://login.microsoftonline.us`.

### Custom authentication

Implement the trait to plug in anything (managed identity, a token from another library, ...):

```rust
use async_trait::async_trait;
use oxipoint::auth::{AccessToken, TokenCredential};
use std::time::Duration;

#[derive(Debug)]
struct MyCredential;

#[async_trait]
impl TokenCredential for MyCredential {
    async fn get_token(&self, scope: &str) -> oxipoint::Result<AccessToken> {
        Ok(AccessToken::new("<token for scope>", Duration::from_secs(3600)))
    }
}
```

`auth::StaticTokenCredential::new("<token>")` covers the simple case of an already acquired token.

## Using Microsoft Graph

`GraphClient` builds requests by chaining handles; the last call (`get`, `list`, `create`, ...)
sends them. Collections follow `@odata.nextLink` automatically.

```rust
use oxipoint::graph::{GraphClient, NewSitePage, NewSubscription};
use serde_json::json;

let graph = GraphClient::new(cred);

// Users
let me = graph.me().get().await?;
let users = graph.users().filter("startswith(displayName,'A')").top(25).list().await?;

// Sites, lists and items
let site = graph.sites().by_path("contoso.sharepoint.com", "/sites/team");
let lists = site.lists().list().await?;
let items = site.list("Tasks").items().expand("fields").list().await?;
let new = site.list("Tasks").items().create(&json!({ "Title": "Write docs" })).await?;
site.list("Tasks").item(&new.id).update(&json!({ "Title": "Docs done" })).await?;
site.list("Tasks").item(&new.id).delete().await?;

// Sites: subsites, columns, content types, permissions, lists, analytics
let subsites = site.sites().await?;
let child = site.site_by_path("child").get().await?;
let columns = site.columns().list().await?;
let types = site.content_types().list().await?;
let perms = site.permissions().list().await?;
let books = site.lists().create(&json!({ "displayName": "Books", "list": { "template": "genericList" } })).await?;
site.list(&books.id).delete().await?;
let followed = graph.followed_sites().await?;
let stats = site.analytics().await?;

// Lists: update, columns, content types, delta, versions, subscriptions
let tasks = site.list("Tasks");
tasks.update(&json!({ "description": "Team tasks" })).await?;
let cols = tasks.columns().list().await?;
let changes = tasks.items().delta(None).await?;                   // first sync
let newer = tasks.items().delta(changes.next_token.as_deref()).await?; // resume
let versions = tasks.item("1").versions().await?;
tasks.subscriptions().create(&NewSubscription::new("https://example.com/hook", "2030-01-01T00:00:00Z")).await?;

// Modern pages: list, read with canvas, create, update, publish, delete
let pages = site.pages();
let all = pages.list().await?;
let home = pages.by_id(&all[0].id).expand("canvasLayout").get().await?;
let page = pages.create(&NewSitePage::new("News.aspx").title("News")).await?;
pages.by_id(&page.id).update(&json!({ "title": "Latest news" })).await?;
pages.by_id(&page.id).publish().await?;
let parts = pages.by_id(&page.id).web_parts().list().await?;
pages.by_id(&page.id).delete().await?;

// Files (document libraries and OneDrive)
let drive = site.drive();                     // or graph.me().drive(), graph.drive("<id>")
let children = drive.root().children().await?;
drive.path("Reports/q1.xlsx").upload(std::fs::read("q1.xlsx")?).await?; // > 4 MiB uses an upload session
let bytes = drive.path("Reports/q1.xlsx").download().await?;
```

Graph v1.0 exposes neither list views nor list permissions; use the SharePoint client
(`sp.list_by_title("Tasks").views()` / `.permissions()`) for those.

Graph has no way to update or delete a site, so the Graph client does not offer it. Use the
SharePoint client (`sp.web().update(..)`, `sp.web().delete()`) for that and for other web features
Graph lacks, such as navigation, features, regional settings and the recycle bin.

### Typed column values

`oxipoint::fields::Fields` builds column values once and serializes them for either back end, so you
do not hand-write lookup or person JSON:

```rust
use oxipoint::fields::Fields;

let f = Fields::new()
    .text("Title", "Write docs")
    .lookup("Project", 3)            // SharePoint: ProjectId = 3, Graph: ProjectLookupId = "3"
    .multi_choice("Tags", vec!["a".into(), "b".into()])
    .url("Link", "https://example.com", Some("Example"));

site.list("Tasks").items().create(&f.for_graph()).await?;
sp.list_by_title("Tasks").items().create(&f.for_sharepoint()).await?;

// Reading: every reader returns None for a missing column or another kind
let item = sp.list_by_title("Tasks").item(7).get().await?;
let project = item.lookup("Project");          // Some(Lookup { id: 3, .. })
let due = item.datetime("Due");                // parsed ISO 8601 (oxipoint::fields::DateTime)
```

Managed metadata has a SharePoint form for a single term only (`taxonomy`) and none on Graph.
Unsupported combinations fail before anything is sent.

SharePoint also validates writes per column and reads formatted values:

```rust
use oxipoint::sharepoint::ValidateOptions;
let results = sp.list_by_title("Tasks").item(7)
    .validate_update(&Fields::new().text("Title", "x"), &ValidateOptions::default()).await?;
let rejected: Vec<_> = results.iter().filter(|r| !r.is_ok()).collect();
let shown = sp.list_by_title("Tasks").item(7).field_values_as_text().await?;
sp.list_by_title("Tasks").item(7).like().await?;
sp.list_by_title("Tasks").item(7).comments().add("Done").await?;
sp.list_by_title("Tasks").item(7).restore_version("2.0").await?;
```

Graph items add metadata (`created_by`, `e_tag`, ...), `drive_item()`, `analytics()`,
`activities()`, `restore_version("1.0")`, `update_if_match(&fields, etag)` and
`items().filter_unindexed("fields/Status eq 'Open'")` (sends the `Prefer` header Graph requires).

| Concept                                       | SharePoint REST                    | Microsoft Graph                     |
| --------------------------------------------- | ---------------------------------- | ----------------------------------- |
| Item id                                       | number (`item(7)`)                 | string (`item("7")`)                |
| Column values                                 | flat on the item                   | under `fields` (`expand("fields")`) |
| Lookup / person                               | `<Col>Id`                          | `<Col>LookupId` (text)              |
| Update                                        | `MERGE` of the item                | `PATCH` of `<item>/fields`          |
| Etag guard                                    | `update_with_etag`                 | `update_if_match`                   |
| Validated write, formatted values             | yes                                | no                                  |
| Likes, comments                               | yes                                | no                                  |
| Created by, drive item, analytics, activities | no (read columns such as `Author`) | yes                                 |
| Restore a version                             | `restore_version("2.0")` (label)   | `restore_version("1.0")` (id)       |

Query options available on collection handles: `select`, `filter`, `expand`, `order_by`, `top`, `search`.
Use `.pages()` instead of `.list()` to consume a collection page by page.

Client settings:

```rust
let graph = GraphClient::builder(cred)
    .base_url("https://graph.microsoft.com/beta") // default: v1.0
    .max_retries(5)
    .build();
```

## Using SharePoint REST

`SharePointClient` is bound to a site URL. Tokens are requested for the tenant host, requests use
the `application/json;odata=nometadata` format, and the form digest needed for writes is fetched
and cached for you.

```rust
use oxipoint::sharepoint::{
    ChangeQuery, ListTemplate, NewList, NewSitePage, NewSubscription, NewWeb, PermissionKind,
    SharePointClient,
};
use serde_json::json;

let sp = SharePointClient::new("https://contoso.sharepoint.com/sites/team", cred)?;

let web = sp.web().get().await?;
let subwebs = sp.web().subwebs().await?;
let lists = sp.lists().select(["Id", "Title", "ItemCount"]).list().await?;

// The web entity: update, subwebs, navigation, columns, content types, users, permissions ...
sp.web().update(&json!({ "Title": "Team site" })).await?;
let child = sp.web().subweb("/sites/team/child");        // every call below also works on `child`
sp.web().create_subweb(&NewWeb::new("Docs", "docs", "STS#3")).await?;
sp.web().navigation().quick_launch().add("Wiki", "/sites/team/wiki").await?;
let fields = sp.web().fields().list().await?;
let doc_type = sp.web().content_types().by_id("0x0101").get().await?;
let me = sp.web().current_user().await?;
let user = sp.web().ensure_user("i:0#.f|membership|a@contoso.com").await?;
let can_edit = sp.web().permissions().effective().await?.has(PermissionKind::EditListItems);
sp.web().permissions().break_inheritance(true, false).await?;   // destructive; resets with reset_inheritance()
sp.web().permissions().add_role_assignment(user.id, 1073741827).await?;
let groups = sp.web().groups().by_id(5).users().list().await?;
sp.web().features().activate("<feature-guid>").await?;
let settings = sp.web().expand("TimeZone").regional_settings().await?;
let bin = sp.web().recycle_bin().list().await?;
let second = sp.web().site_recycle_bin().second_stage().list().await?;   // site collection bin, stage 2
sp.web().site_recycle_bin().restore_many(&["<guid-1>", "<guid-2>"]).await?;

// Modern pages: list, read with canvas content, create, update, publish, delete
let pages = sp.web().pages();
let all = pages.list().await?;
let home = pages.by_id(all[0].id).select(["Id", "Title", "CanvasContent1"]).get().await?;
let news = pages.create(&NewSitePage::new("News.aspx").title("News")).await?;
pages.by_id(news.id).update(&json!({ "Title": "Latest news" })).await?;
pages.by_id(news.id).publish().await?;
pages.by_id(news.id).delete().await?;

// Create, update and delete lists
let books = sp.lists().create(&NewList::new("Books", ListTemplate::GenericList).description("Reading list")).await?;
sp.list_by_title("Books").update(&json!({ "Description": "Updated" })).await?;
sp.list_by_title("Books").recycle().await?;                       // or .delete() to remove for good

// Fields, views, content types
let status = sp.list_by_title("Tasks").fields().by_internal_name("Status").get().await?;
sp.list_by_title("Tasks").fields().create(&json!({ "Title": "Due", "FieldTypeKind": 4 })).await?;
let all = sp.list_by_title("Tasks").views().by_title("All Items").fields().await?;
sp.list_by_title("Tasks").content_types().add_from_web("0x0100...").await?;

// CAML queries, counts, changes, attachments, versions, permissions, webhooks
let list = sp.list_by_title("Tasks");
let hits = list.items().caml("<View><Query><Where>...</Where></Query></View>").await?.collect_all().await?;
let n = list.items().filter("Status eq 'Open'").count().await?;
let ch = list.get_changes(&ChangeQuery::items()).await?;
list.item(7).attachments().add("a.txt", "hello").await?;
list.item(7).update_with_etag(&json!({ "Title": "x" }), "\"3\"").await?;  // fails if the item changed
list.item(7).permissions().break_inheritance(false, false).await?;
list.subscriptions().add(&NewSubscription::new("https://example.com/hook", "2030-01-01T00:00:00Z")).await?;

// List items
let tasks = sp.list_by_title("Tasks");
let open = tasks.items().filter("Status eq 'Open'").select(["Id", "Title"]).top(50).list().await?;
let created = tasks.items().create(&json!({ "Title": "New task" })).await?;
tasks.item(created.id).update(&json!({ "Title": "Renamed" })).await?;
tasks.item(created.id).delete().await?;

// Folders and files, addressed by server-relative URL
let folder = sp.folder("/sites/team/Shared Documents/Reports");
let files = folder.files().await?;
folder.upload("hello.txt", "hello", true).await?;          // overwrite = true
let data = sp.file("/sites/team/Shared Documents/Reports/hello.txt").download().await?;
```

Tips:

- When you use `select`, include `Id` if you need it. Missing ids are returned as empty/0 rather
  than failing.
- List item columns other than `Id` are in `ListItem::fields` (a JSON map), keyed by internal name.
- Names containing spaces, quotes or `#` are escaped for you.
- Updates and deletes are sent as `POST` with `X-HTTP-Method: MERGE|DELETE` and `IF-MATCH: *`.

## Files, search, sharing and batching

The core of these examples is compiled as a doctest on `GraphClient` and `SharePointClient`.

```rust
use reqwest::Method;
use oxipoint::graph::{DriveItemUpdate, GraphBatchRequest, LinkOptions, LinkScope, LinkType};
use std::time::Duration;

let drive = graph.drive("<drive-id>");

// Folders, rename/move, copy (asynchronous on the service) and streamed download
let folder = drive.root().create_folder("Reports", None).await?;
let report = drive.path("Reports/q1.xlsx");
report.update(&DriveItemUpdate::new().name("q1-final.xlsx")).await?;
report.copy("<drive-id>", &folder.id, Some("backup.xlsx")).await?
    .wait(Duration::from_secs(2), Duration::from_secs(120)).await?;
let mut out = tokio::fs::File::create("q1.xlsx").await?;
report.download_to(&mut out).await?;          // never holds the whole file in memory

// Everything that changed in a drive since the last sync
let sync = drive.delta(None).await?;
let later = drive.delta(sync.next_token.as_deref()).await?;

// Sharing links, invitations and permissions (Microsoft Graph)
let link = report.create_link(&LinkOptions::new(LinkType::View).scope(LinkScope::Organization)).await?;
println!("{:?}", link.link_url());
let item = graph.shared_item("https://contoso.sharepoint.com/:x:/s/team/abc").get().await?;

// Search
let hits = graph.search().entity_types(["driveItem"]).query("budget").all().await?;

// Batching: up to 20 calls in one request, one result each. A failed call does not fail the batch.
let mut batch = graph.batch();
batch.add(GraphBatchRequest::new(Method::GET, "/me"))?;
batch.add(GraphBatchRequest::new(Method::GET, "/me/drive"))?;
for result in batch.send().await? {
    let value: serde_json::Value = result.json()?;   // a failed call is an Err here
}
```

SharePoint REST has the equivalent file operations, plus check-out and publishing:

```rust
use oxipoint::sharepoint::{CheckInKind, SharePointBatchRequest};

let file = sp.file("/sites/team/Shared Documents/plan.docx");
file.checkout().await?;
file.checkin("Updated the schedule", CheckInKind::Major).await?;
file.copy_to("/sites/team/Archive/plan.docx", false).await?;   // false: do not overwrite
file.rename("plan-v2.docx").await?;
let versions = file.versions().await?;
file.restore_version("1.0").await?;

let everything = sp.folder("/sites/team/Shared Documents").list_recursive(false).await?;

let rows = sp.search("contract").select(["Title", "Path"]).all().await?;
println!("{:?}", rows[0].get("Title"));

// One round trip: a read, and two writes that succeed or fail together
let mut batch = sp.batch();
batch.add(SharePointBatchRequest::new(Method::GET, "/web/title"))?;
batch.add_changeset([
    SharePointBatchRequest::new(Method::POST, "/web/lists/getbytitle('Tasks')/items")
        .json(&serde_json::json!({ "Title": "A" }))?,
    SharePointBatchRequest::new(Method::POST, "/web/lists/getbytitle('Tasks')/items")
        .json(&serde_json::json!({ "Title": "B" }))?,
])?;
let results = batch.send().await?;
```

Notes:

- `download_to` takes any `tokio::io::AsyncWrite`. The same streaming is available on the generic
  builder as `request(...).copy_to(&mut writer)` and `send_stream()`.
- Inside a batch, a throttled call (429) is reported with `BatchResult::retry_after()` and not
  retried, because part of a batch cannot be replayed safely. Resend those calls in a new batch.
- SharePoint search and batch use the REST wire formats documented by Microsoft. The multipart batch
  and `SP.MoveCopyUtil` folder copy are tested against recorded response shapes, so try them on your
  tenant before depending on them.
- SharePoint REST has no typed sharing calls. Use Graph's `create_link` and `invite`, or the generic
  request builder.

## What is covered

| Entity            | SharePoint REST (`sp`)                                                                                                                        | Microsoft Graph (`graph`)                                                                                      |
| ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Web / site        | read, update, delete, subwebs, navigation, users, groups, permissions, features, regional settings, recycle bin                               | read, subsites, columns, content types, permissions, analytics, followed sites (no update or delete)           |
| Lists             | create, update, delete, recycle, fields, views, content types, permissions, webhooks                                                          | create, update, delete, columns, content types, subscriptions (no views or permissions)                        |
| Items             | CRUD, CAML, count, changes, attachments, versions, validated writes, formatted values, likes, comments                                        | CRUD, delta, versions, metadata, drive item, analytics, activities, etag guard, unindexed filter               |
| Modern pages      | list, read (canvas content), create, update, publish, delete                                                                                  | list, read (canvas layout), create, update, publish, delete, web parts                                         |
| Files and folders | read, upload, download (also streamed), folders, copy, move, rename, recursive listing, check-out and check-in, publish and approve, versions | drives, upload (resumable), download (also streamed), create folder, rename, move, copy, versions, drive delta |
| Sharing           | not covered (use the generic builder or Graph)                                                                                                | sharing links, invitations, item permissions, open a sharing URL                                               |
| Search            | query with select, sort, refiners and paging                                                                                                  | query over drive items, list items and sites, with paging                                                      |
| Batching          | `$batch` with changesets                                                                                                                      | `$batch` (20 calls, dependencies)                                                                              |
| Users             | current user, site users, groups, ensure user                                                                                                 | me, users                                                                                                      |

Anything else is reachable with the generic request builder below.

## Calling endpoints that have no typed API

Both clients expose `request(method, path)`. It takes a path relative to the client's base URL
(`/me/messages`, `/web/currentuser`) or an absolute URL on the **same host**, and returns a builder
with `query`, `options`, `header`, `json`, `body`, `send`, `send_json`, `send_stream`, `copy_to`
and `pager`.
Authentication, throttling retries and error mapping all apply.

```rust
use reqwest::Method;

// Typed response
let recent: serde_json::Value = graph
    .request(Method::GET, "/me/messages")
    .query("$top", "5")
    .send_json()
    .await?;

// Send mail
graph.request(Method::POST, "/me/sendMail")
    .json(&serde_json::json!({
        "message": {
            "subject": "Hi",
            "body": { "contentType": "Text", "content": "Hello" },
            "toRecipients": [{ "emailAddress": { "address": "a@contoso.com" } }]
        }
    }))
    .send()
    .await?;

// Any paged collection
#[derive(serde::Deserialize)]
struct Group { id: String }
let groups: Vec<Group> = graph.request(Method::GET, "/groups").pager()?.collect_all().await?;

// SharePoint
let user: serde_json::Value = sp.request(Method::GET, "/web/currentuser").send_json().await?;
```

For writes to SharePoint through the generic builder, send an `X-RequestDigest` header yourself
(`POST /contextinfo` returns the value). The typed write methods do this automatically.

## Errors, retries and paging

All fallible calls return `oxipoint::Result<T>`. `oxipoint::Error` is `#[non_exhaustive]`:

| Variant       | Meaning                                                                                                                                        |
| ------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `Auth`        | The identity platform refused a token (`code` such as `invalid_grant`, message).                                                               |
| `Config`      | Bad local input: certificate, URL, or a request aimed at a foreign host.                                                                       |
| `Http`        | Non-2xx service response: `status`, service `code`, `message`, `request_id`. A failed Graph copy is reported as status 500, code `copyFailed`. |
| `Transport`   | Network or TLS failure.                                                                                                                        |
| `Deserialize` | The body did not match the expected type.                                                                                                      |
| `Io`          | Local I/O: the browser flow's loopback listener, a writer given to `download_to`, or a copy wait that timed out.                               |

```rust
match graph.me().get().await {
    Ok(me) => println!("{:?}", me.display_name),
    Err(oxipoint::Error::Http { status: 403, code, .. }) => eprintln!("missing permission: {code:?}"),
    Err(e) => eprintln!("failed: {e}"),
}
```

- **Throttling**: 429 responses (and 503/504 for idempotent requests; a 503 with `Retry-After` always)
  are retried up to 3 times (configurable), waiting for `Retry-After` when present, 1 second otherwise. After that the last error is returned.
- **Paging**: `list()` collects every page; `pages()` / `pager()` return a `Pager` with
  `next_page()` and `collect_all()`. Graph, SharePoint light and verbose continuation links work.
- **Credential safety**: the bearer token is only ever sent to the client's own host. A request or
  continuation link pointing elsewhere fails with `Error::Config`. The only deliberate exceptions are
  pre-authorized URLs, which are called without a token: the upload session of large Graph uploads and
  the monitor URL of a Graph copy.

## Cargo features

All features are enabled by default.

| Feature            | Enables                                           |
| ------------------ | ------------------------------------------------- |
| `graph`            | `oxipoint::graph`                                 |
| `sharepoint`       | `oxipoint::sharepoint`                            |
| `auth-password`    | `PasswordCredential`                              |
| `auth-device-code` | `DeviceCodeCredential`                            |
| `auth-browser`     | `BrowserCredential` (adds the `open` crate)       |
| `auth-certificate` | `CertificateCredential` (adds JWT and PFX crates) |

```toml
oxipoint = { git = "...", default-features = false, features = ["sharepoint", "auth-certificate"] }
```
