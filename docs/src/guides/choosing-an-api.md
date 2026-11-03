# Choosing between Graph and SharePoint

Both APIs reach the same data. Use this chapter to decide which one to call, and how the
differences show up in code.

## Rules of thumb

- **Default to Graph** for new work: one endpoint for the whole of Microsoft 365, modern
  features (delta, sharing links, search across entities, resumable uploads).
- **Use SharePoint REST** when Graph has no equivalent: views, permissions, navigation, features,
  recycle bin, CAML, validated writes, check-out/in, likes and comments.
- **Mix them.** Both clients can share credentials, and nothing stops a program from using each
  for what it does best.

## Side by side

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
| Addressing files                              | server-relative URL                | drive + path or id                  |
| Incremental sync                              | `get_changes`                      | `delta`                             |
| App-only auth                                 | certificate only                   | certificate only                    |

## The same task in both

Create a task and mark it done.

**Graph**

```rust
let tasks = graph.sites().by_path("contoso.sharepoint.com", "/sites/team").list("Tasks");
let item = tasks.items().create(&serde_json::json!({ "Title": "Ship it" })).await?;
tasks.item(&item.id).update(&serde_json::json!({ "Status": "Done" })).await?;
```

**SharePoint**

```rust
let tasks = sp.list_by_title("Tasks");
let item = tasks.items().create(&serde_json::json!({ "Title": "Ship it" })).await?;
tasks.item(item.id).update(&serde_json::json!({ "Status": "Done" })).await?;
```

To write one definition that works on both, use [`Fields`](fields.md).

## Quick decision table

| I need to...                              | Use       |
| ----------------------------------------- | --------- |
| Read OneDrive or a document library       | Graph     |
| Upload a file bigger than 250 MB          | Graph (`upload_large`) |
| Create sharing links or invite people     | Graph     |
| Search across files, items and sites      | Graph     |
| Get changes since last sync               | Graph `delta` |
| Query with CAML or on many columns        | SharePoint|
| Manage permissions / break inheritance    | SharePoint|
| Edit navigation, features, site settings  | SharePoint|
| Work with the recycle bin                 | SharePoint|
| Update or delete a site                   | SharePoint|
| Check files out and in                    | SharePoint|
| Anything else with no typed method        | [Generic request](generic-requests.md) |
