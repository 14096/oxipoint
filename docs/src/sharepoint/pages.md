# Modern pages

```rust
use oxipoint::sharepoint::NewSitePage;
use serde_json::json;

let pages = sp.web().pages();
```

## List and read

```rust
let all = pages.list().await?;
let home = pages
    .by_id(all[0].id)
    .select(["Id", "Title", "CanvasContent1"])
    .get()
    .await?;
```

`CanvasContent1` is the raw JSON describing sections and web parts.

## Create, update, publish, delete

```rust
let news = pages.create(&NewSitePage::new("News.aspx").title("News")).await?;
pages.by_id(news.id).update(&json!({ "Title": "Latest news" })).await?;
pages.by_id(news.id).publish().await?;
pages.by_id(news.id).delete().await?;
```

Page ids are numbers (list item ids of *Site Pages*).

For structured access to the canvas (sections, columns, web parts), the
[Graph pages API](../graph/pages.md) is the better choice.
