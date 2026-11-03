# Modern pages

Modern SharePoint pages (`.aspx` in *Site Pages*) are available through `site.pages()`.

```rust
use oxipoint::graph::NewSitePage;
use serde_json::json;

let pages = site.pages();
```

## List and read

```rust
let all = pages.list().await?;

// Read with the canvas layout (sections, columns, web parts)
let home = pages.by_id(&all[0].id).expand("canvasLayout").get().await?;
```

## Create, update and publish

A new page is a draft. Publish it to make it visible to readers:

```rust
let page = pages.create(&NewSitePage::new("News.aspx").title("News")).await?;

pages.by_id(&page.id).update(&json!({ "title": "Latest news" })).await?;
pages.by_id(&page.id).publish().await?;
```

## Web parts

```rust
let parts = pages.by_id(&page.id).web_parts().list().await?;
```

## Delete

```rust
pages.by_id(&page.id).delete().await?;
```
