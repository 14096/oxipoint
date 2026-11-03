# Overview

`SharePointClient` talks to the **SharePoint REST API** (`/_api`) of **one site**. Reach for it
when you need things Graph does not offer: list views, permissions, navigation, features, the
recycle bin, CAML queries, validated writes, check-out/check-in and more.

```rust
use oxipoint::sharepoint::SharePointClient;

let sp = SharePointClient::new("https://contoso.sharepoint.com/sites/team", cred)?;
```

`new` returns `Err(Error::Config)` for a malformed or relative URL.

## What the client does for you

- Requests tokens for the tenant host (`https://contoso.sharepoint.com/.default`).
- Uses the lightweight `application/json;odata=nometadata` format.
- Fetches and **caches the form digest** required by writes.
- Sends updates and deletes as `POST` with `X-HTTP-Method: MERGE|DELETE` and `IF-MATCH: *`.
- Escapes names containing spaces, quotes or `#` in paths.

## Map of the API

```text
sp
├── web()                  ─ the site: get/update/delete, subwebs, navigation(), features(),
│                            users(), groups(), permissions(), fields(), content_types(),
│                            pages(), recycle_bin(), site_recycle_bin(), regional_settings()
├── lists()                ─ list/create lists
├── list_by_title("T")     ─ items(), item(id), fields(), views(), content_types(),
│   list_by_id("guid")       subscriptions(), permissions(), get_changes()
├── folder("/sites/..")    ─ folders(), files(), upload(), list_recursive()
├── file("/sites/..")      ─ download, copy/move/rename, check-out/in, versions
├── search("text")         ─ SharePoint search
├── batch()                ─ $batch with changesets
└── request(method, path)  ─ generic builder
```

## Addressing conventions

- Lists: by **title** (`list_by_title`) or id (`list_by_id`).
- Items: by **numeric id** (`item(7)`).
- Files and folders: by **server-relative URL** (`/sites/team/Shared Documents/a.txt`).
- Sub-webs: `sp.web().subweb("/sites/team/child")` returns a handle with the same API.
