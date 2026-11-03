# Files and folders

SharePoint addresses files and folders by their **server-relative URL**: the path starting at the
site, such as `/sites/team/Shared Documents/Reports/q1.xlsx`.

## Folders

```rust
let folder = sp.folder("/sites/team/Shared Documents/Reports");

let info = folder.get().await?;
let subfolders = folder.folders().await?;
let files = folder.files().await?;

folder.create().await?;                         // create this folder
folder.rename("Reports-2026").await?;
folder.move_to("/sites/team/Archive/Reports").await?;
folder.copy_to("/sites/team/Archive/Reports-copy").await?;
folder.recycle().await?;                        // recoverable; returns the recycle bin item id
folder.delete().await?;                         // permanent
```

Everything inside a folder, recursively:

```rust
let everything = sp.folder("/sites/team/Shared Documents").list_recursive(false).await?;
```

The boolean controls whether system folders (such as `Forms`) are included.

## Upload

```rust
let file = folder.upload("hello.txt", "hello", true).await?;    // overwrite = true
let bytes = std::fs::read("q1.xlsx")?;
folder.upload("q1.xlsx", bytes, false).await?;                  // overwrite = false: fail if it exists
```

## Download

```rust
let f = sp.file("/sites/team/Shared Documents/Reports/hello.txt");

let data = f.download().await?;                     // in memory

let mut out = tokio::fs::File::create("hello.txt").await?;
f.download_to(&mut out).await?;                     // streamed, never fully in memory
```

## Move, copy, rename, delete

```rust
let file = sp.file("/sites/team/Shared Documents/plan.docx");

file.copy_to("/sites/team/Archive/plan.docx", false).await?;   // false: do not overwrite
file.move_to("/sites/team/Archive/plan2.docx", true).await?;
file.rename("plan-v2.docx").await?;
file.delete().await?;
```

## Check-out and check-in

Libraries that require check-out make you lock a file before editing:

```rust
use oxipoint::sharepoint::CheckInKind;

file.checkout().await?;
// ... upload a new version ...
file.checkin("Updated the schedule", CheckInKind::Major).await?;

file.undo_checkout().await?;        // abandon changes
```

## Publishing and approval

```rust
file.publish("Ready for readers").await?;
file.unpublish("Pulled back").await?;
file.approve("Looks good").await?;
file.deny("Needs changes").await?;
```

## Versions

```rust
let versions = file.versions().await?;
let old = file.download_version(versions[0].id).await?;
file.restore_version("1.0").await?;
file.delete_version("2.0").await?;
```

## Files as list items

A file in a library is also a list item. Read its columns through `item(id).file()` or
`item(id).get()` on the library's list.
