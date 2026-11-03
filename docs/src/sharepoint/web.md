# Web, navigation and features

In SharePoint REST, a **web** is a site (or subsite). `sp.web()` is the web the client is bound to.

## Read and update

```rust
use serde_json::json;

let web = sp.web().get().await?;
println!("{:?}", web.title);

// Only some fields
let web = sp.web().select(["Title", "Url", "Id"]).get().await?;

sp.web().update(&json!({ "Title": "Team site" })).await?;
```

Deleting a web is destructive:

```rust
sp.web().delete().await?;
```

## Site information

```rust
let info = sp.web().site_info().await?;
let parent = sp.web().parent_web().await?;
let root = sp.web().root_folder().await?;
```

## Subwebs

```rust
use oxipoint::sharepoint::NewWeb;

let subwebs = sp.web().subwebs().await?;
sp.web().create_subweb(&NewWeb::new("Docs", "docs", "STS#3")).await?;

let child = sp.web().subweb("/sites/team/child");     // every call below also works on `child`
```

> A subweb handle exposes the same web-level API (`get`, `navigation`, `permissions`, ...).

## Navigation

Two trees exist: the top navigation bar and the quick launch (left menu).

```rust
let quick = sp.web().navigation().quick_launch();
quick.add("Wiki", "/sites/team/wiki").await?;

let nodes = sp.web().navigation().top().list().await?;
let node = sp.web().navigation().node(1042);
node.update(&json!({ "Title": "Renamed" })).await?;
let kids = node.children().list().await?;
node.delete().await?;
```

## Site features

```rust
sp.web().features().activate("<feature-guid>").await?;
sp.web().features().deactivate("<feature-guid>").await?;
```

## Regional settings

```rust
let settings = sp.web().expand("TimeZone").regional_settings().await?;
```

## Site columns and content types

```rust
let fields = sp.web().fields().list().await?;
let doc_type = sp.web().content_types().by_id("0x0101").get().await?;
```

## Templates

```rust
let list_templates = sp.web().list_templates().await?;
let web_templates = sp.web().web_templates(1033).await?;   // 1033 = English (US)
```
