# Recipes

Complete, task-sized examples. They assume `sp` and `graph` are set up as in
[Getting started](../getting-started/first-program.md).

## Export a list to CSV

```rust
use serde_json::Value;
use std::fmt::Write;

let items = sp.list_by_title("Tasks").items().select(["Id", "Title", "Status"]).list().await?;

let mut csv = String::from("id,title,status\n");
for item in items {
    let cell = |k: &str| match item.fields.get(k) {
        Some(Value::String(s)) => s.replace('"', "\"\""),
        Some(v) => v.to_string(),
        None => String::new(),
    };
    writeln!(csv, "{},\"{}\",\"{}\"", item.id, cell("Title"), cell("Status")).unwrap();
}
std::fs::write("tasks.csv", csv)?;
```

## Bulk create items without hammering the service

Batch 20 at a time on Graph:

```rust
use oxipoint::graph::GraphBatchRequest;
use reqwest::Method;

let site_id = "<site-id>";
let list_id = "<list-id>";
let titles: Vec<String> = (1..=100).map(|i| format!("Item {i}")).collect();

for chunk in titles.chunks(20) {
    let mut batch = graph.batch();
    for t in chunk {
        batch.add(
            GraphBatchRequest::new(Method::POST, format!("/sites/{site_id}/lists/{list_id}/items"))
                .json(&serde_json::json!({ "fields": { "Title": t } }))?,
        )?;
    }
    for r in batch.send().await? {
        if let Some(wait) = r.retry_after() {
            eprintln!("throttled, retry after {wait:?}");
        }
    }
}
```

## Upload a whole directory

```rust
let drive = site.drive();
for entry in std::fs::read_dir("./out")? {
    let path = entry?.path();
    if path.is_file() {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        drive.path(&format!("Uploads/{name}")).upload(std::fs::read(&path)?).await?;
        println!("uploaded {name}");
    }
}
```

## Mirror a library (incremental)

```rust
use std::fs;

let state_file = "delta.token";
let token = fs::read_to_string(state_file).ok();

let page = drive.delta(token.as_deref()).await?;
for item in &page.items {
    println!("changed: {:?}", item.name);
}
if let Some(t) = page.next_token {
    fs::write(state_file, t)?;
}
```

## Find a list or create it

```rust
use oxipoint::sharepoint::{ListTemplate, NewList};

async fn ensure_list(sp: &oxipoint::sharepoint::SharePointClient, title: &str) -> oxipoint::Result<()> {
    match sp.list_by_title(title).get().await {
        Ok(_) => Ok(()),
        Err(e) if e.status() == Some(404) => {
            sp.lists().create(&NewList::new(title, ListTemplate::GenericList)).await?;
            Ok(())
        }
        Err(e) => Err(e),
    }
}
```

## Retry on a conflict

Use the etag to avoid lost updates, and retry when someone else won:

```rust
for _ in 0..3 {
    let item = graph_item.get().await?;
    let etag = item.e_tag.clone().unwrap_or_default();
    match graph_item.update_if_match(&serde_json::json!({ "Title": "x" }), &etag).await {
        Ok(_) => break,
        Err(e) if e.status() == Some(412) => continue,   // precondition failed: re-read and retry
        Err(e) => return Err(e),
    }
}
```

## Limit parallelism

```rust
use std::sync::Arc;
use tokio::sync::Semaphore;

let sem = Arc::new(Semaphore::new(5));
let mut jobs = Vec::new();
for id in 1..=50 {
    let (sp, sem) = (sp.clone(), sem.clone());
    jobs.push(tokio::spawn(async move {
        let _permit = sem.acquire().await.unwrap();
        sp.list_by_title("Tasks").item(id).get().await
    }));
}
```

(`sp` here is an `Arc<SharePointClient>`.)

## Find who can edit

```rust
use oxipoint::sharepoint::PermissionKind;

let perms = sp.list_by_title("Tasks").permissions().effective().await?;
println!("can edit items: {}", perms.has(PermissionKind::EditListItems));
```
