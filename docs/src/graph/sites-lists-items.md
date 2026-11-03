# Sites, lists and items

## Finding a site

There are several ways to address a site; pick the one you have:

```rust
let sites = graph.sites();

let root = sites.root();                                                    // tenant root site
let team = sites.by_path("contoso.sharepoint.com", "/sites/team");          // host + server-relative path
let by_id = sites.by_id("contoso.sharepoint.com,<site-guid>,<web-guid>");   // full Graph id
let found = sites.search("marketing").await?;                              // Vec<Site>
```

Read it:

```rust
let info = team.get().await?;
println!("{:?} {:?}", info.display_name, info.web_url);
```

### Subsites, followed sites and analytics

```rust
let subsites = team.sites().await?;
let child = team.site_by_path("child").get().await?;
let followed = graph.followed_sites().await?;
let stats = team.analytics().await?;
```

### Site columns, content types and permissions

```rust
let columns = team.columns().list().await?;
let types = team.content_types().list().await?;
let perms = team.permissions().list().await?;
```

## Lists

```rust
use serde_json::json;

let lists = team.lists().list().await?;

// Create
let books = team
    .lists()
    .create(&json!({ "displayName": "Books", "list": { "template": "genericList" } }))
    .await?;

// Update and delete
team.list(&books.id).update(&json!({ "description": "Reading list" })).await?;
team.list(&books.id).delete().await?;
```

`list(id_or_name)` accepts the list id or its display name.

### List columns and content types

```rust
let tasks = team.list("Tasks");
let cols = tasks.columns().list().await?;
let types = tasks.content_types().list().await?;
```

## Items

List items store their column values under `fields`, so ask Graph to expand them:

```rust
let tasks = team.list("Tasks");

let items = tasks.items().expand("fields").list().await?;
for item in &items {
    let title = item.fields.as_ref().and_then(|f| f.get("Title"));
    println!("{}: {:?}", item.id, title);
}
```

Select just a few columns with Graph's expand syntax:

```rust
let items = tasks.items().expand("fields(select=Title,Status)").list().await?;
```

### Create, update, delete

```rust
let new = tasks.items().create(&json!({ "Title": "Write docs" })).await?;
tasks.item(&new.id).update(&json!({ "Title": "Docs done" })).await?;
tasks.item(&new.id).delete().await?;
```

Item ids are **strings** in Graph (`item("7")`).

### Filtering

```rust
// Indexed columns
let open = tasks.items().expand("fields").filter("fields/Status eq 'Open'").list().await?;

// Non-indexed columns need a special header; this method sends it for you
let slow = tasks.items().expand("fields").filter_unindexed("fields/Owner eq 'ann'").list().await?;
```

> Filtering on non-indexed columns can be slow or fail on large lists (over the 5,000-item list
> view threshold). Index the column in SharePoint when you can.

### Optimistic concurrency

Pass the item's e-tag so the update fails if someone else changed it meanwhile:

```rust
let item = tasks.item("7").get().await?;
let etag = item.e_tag.clone().unwrap_or_default();
tasks.item("7").update_if_match(&json!({ "Title": "x" }), &etag).await?;
```

### Metadata, versions and activity

```rust
let item = tasks.item("7").get().await?;
println!("{:?} {:?}", item.created_by, item.last_modified_date_time);

let versions = tasks.item("7").versions().await?;
tasks.item("7").restore_version("1.0").await?;

let stats = tasks.item("7").analytics().await?;
let acts = tasks.item("7").activities().await?;
let file = tasks.item("7").drive_item();      // for document libraries: the underlying file
```

## Typed column values

Instead of hand-writing lookup and person JSON, use [`Fields`](../guides/fields.md):

```rust
use oxipoint::fields::Fields;

let f = Fields::new().text("Title", "Write docs").lookup("Project", 3);
tasks.items().create(&f.for_graph()).await?;
```
