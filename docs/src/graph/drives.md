# Drives and files

A **drive** is a document library or a OneDrive. Files and folders are **drive items**.

## Getting a drive

```rust
let drive = site.drive();                 // a site's default document library
let mine = graph.me().drive();            // the signed-in user's OneDrive
let other = graph.drive("<drive-id>");    // any drive by id
let libraries = site.drives().await?;     // every library on a site
```

## Browsing

```rust
let children = drive.root().children().await?;
for item in children {
    let kind = if item.folder.is_some() { "folder" } else { "file" };
    println!("{kind}: {:?} ({:?} bytes)", item.name, item.size);
}

let report = drive.path("Reports/q1.xlsx");   // address by path
let same = drive.item("<item-id>");            // address by id
let meta = report.get().await?;
```

## Upload

```rust
drive.path("Reports/q1.xlsx").upload(std::fs::read("q1.xlsx")?).await?;
```

Content up to 4 MiB is sent in one request. Larger content automatically uses a **resumable upload
session**. To control the chunk size yourself (must be a multiple of 320 KiB):

```rust
use bytes::Bytes; // add `bytes = "1"` to your dependencies

let data = Bytes::from(std::fs::read("big.zip")?);
drive.path("Archive/big.zip").upload_large(data, 10 * 320 * 1024).await?;
```

## Download

```rust
let bytes = drive.path("Reports/q1.xlsx").download().await?;      // whole file in memory
```

For large files, **stream to a writer** without holding them in memory:

```rust
let mut out = tokio::fs::File::create("q1.xlsx").await?;
let written = drive.path("Reports/q1.xlsx").download_to(&mut out).await?;
```

`download_to` takes any `tokio::io::AsyncWrite`.

## Folders, rename and move

```rust
use oxipoint::graph::{ConflictBehavior, DriveItemUpdate};

let folder = drive.root().create_folder("Reports", None).await?;                 // fails if it exists
let safe = drive.root().create_folder("Reports", Some(ConflictBehavior::Rename)).await?;

let report = drive.path("Reports/q1.xlsx");
report.update(&DriveItemUpdate::new().name("q1-final.xlsx")).await?;
```

## Copy

Copying is **asynchronous on the service**. `copy` returns a handle you can follow:

```rust
use std::time::Duration;

let progress = report
    .copy("<drive-id>", &folder.id, Some("backup.xlsx"))
    .await?
    .wait(Duration::from_secs(2), Duration::from_secs(120)) // poll every 2s, give up after 2min
    .await?;
```

If the copy fails you get an `Error::Http` with status 500 and code `copyFailed`. If the wait runs
out you get `Error::Io`.

## Versions

```rust
let versions = report.versions().await?;
let old = report.download_version(&versions[1].id).await?;
report.restore_version(&versions[1].id).await?;
```

## Delete

```rust
report.delete().await?;
```

## Delta: what changed since last time

```rust
let sync = drive.delta(None).await?;                           // first call: the full state
// store sync.next_token
let later = drive.delta(sync.next_token.as_deref()).await?;    // later: only changes
```

See [Delta sync and subscriptions](delta-subscriptions.md).
