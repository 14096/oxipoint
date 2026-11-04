# Changelog

All notable changes to this project are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project
uses [Semantic Versioning](https://semver.org/).

## [0.0.1-beta7] - 2026-11-04

### Added

- **`filter!` macro and `Filter` builder** (`oxipoint::filter`): build OData `$filter`
  expressions with Rust-like syntax; string values are quoted and escaped, `&&` binds tighter
  than `||`, and parentheses group. Results can be passed to any `filter` method.
- **Error helpers**: `Error::code`, `request_id`, `is_not_found`, `is_unauthorized`,
  `is_forbidden`, `is_conflict` and `is_throttled`.
- **`tracing` feature** (off by default): the transport logs each request (method, host, path,
  status, attempt, elapsed time), retries and failures with the service request id. Query
  strings and tokens are never logged.

## [0.0.1-beta6] - 2026-11-02

### Changed

- Renamed the crate from `rsp` to `oxipoint`.

## [0.0.1-beta5] - 2026-11-01

### Added

- **Recycle bin**: site collection bin (`web().site_recycle_bin()`), first and second stage
  selection, `move_to_second_stage`, bulk `restore_many`, `delete_many` and
  `move_many_to_second_stage`, and more `RecycleBinItem` metadata (item type, item state, author,
  deleted by).

### Fixed

- Bodyless `POST`, `PUT` and `PATCH` requests now send `Content-Length: 0`; SharePoint answered
  them with HTTP 411.
- `ListItem` and `Comment` accept `Id`/`ID` (and other case variants) returned together, which
  failed with "duplicate field" when creating an item.
- `FileInfo` and `FolderInfo` no longer fail when `$select` leaves out `Name` or
  `ServerRelativeUrl`, and `FileInfo.length` accepts a number as well as a string.
- `503` (without `Retry-After`) and `504` are no longer retried for non-idempotent requests, so a
  create cannot be duplicated; `429` and `503` with `Retry-After` are still retried.

## [0.0.1-beta4] - 2026-10-31

### Added

- **Streaming downloads**: `Transport::send_stream`, `StreamResponse`, `RequestBuilder::send_stream`
  and `copy_to`, and typed `download_to(writer)` on Graph drive items and SharePoint files.
  `download() -> Bytes` is unchanged.
- **Graph drive items**: create folder (with conflict behavior), `update` (rename, move,
  description), asynchronous `copy` with a `CopyOperation` monitor, versions (list, restore,
  download), drive `delta` with token resume, and more `DriveItem` metadata (parent reference,
  timestamps, identities, ETag, cTag, deleted facet, MIME type).
- **SharePoint files and folders**: file copy, move and rename; folder delete, recycle, move, copy
  and rename; `list_recursive`; check-out, check-in, undo check-out, publish, unpublish, approve and
  deny; file versions (list, read, restore, delete, download); file properties through `expand`.
- **Batching**: `GraphClient::batch` (`$batch`, 20 calls, dependencies) and
  `SharePointClient::batch` (multipart `$batch` with changesets). A failing sub-request does not
  fail the batch; each one has a `BatchResult`.
- **Search**: `GraphClient::search` (`/search/query`) and `SharePointClient::search`
  (`/_api/search/query`) with typed results and `all()` paging.
- **Graph sharing**: `create_link`, `invite`, item `permissions`, and `GraphClient::shared_item` to
  open a sharing URL. `Permission` gained `has_password`, `link_url`, `link_type` and `link_scope`.

## [0.0.1-beta3] - 2026-10-31

### Added

- **Graph site pages**: `site.pages()` lists, reads (with `canvasLayout`), creates, updates,
  publishes and deletes modern pages, and lists a page's web parts.
- **SharePoint site pages**: `web().pages()` lists, reads (with `CanvasContent1`), creates,
  updates, publishes and deletes modern pages through `_api/sitepages`.

### Changed

- The Graph and SharePoint clients' internal modules now follow their resource hierarchies
  (Graph: `sites`, `lists`, `drives`, `users`; SharePoint: `web`, `fields`, `content_types`,
  `principals`, `lists`, `files`, `changes`, `pages`). Public paths are unchanged.

## [0.0.1-beta2] - 2026-10-31

### Added

- **`oxipoint::fields`**: typed list item column values (`Fields`, `FieldValue`) that serialize for
  SharePoint (`for_sharepoint()`) or Graph (`for_graph()`), typed readers (`text`, `number`,
  `bool`, `datetime`, `url`, `lookup`, `person`, `taxonomy`) on both `ListItem` types, and a
  dependency-free ISO 8601 `DateTime`. Unsupported kind and back-end pairs fail before sending.
- **SharePoint web**: update and delete, create subweb (`NewWeb`) and subweb by URL (every
  sub-handle also works on subwebs), navigation, fields, content types, users and groups, role
  definitions and assignments, effective permissions with `BasePermissions::has`, break and reset
  inheritance, features, regional settings, recycle bin, parent web, site collection info, root
  folder, list and web templates. `Web` gained language, timestamps, template and master page URL.
- **SharePoint lists**: create (`NewList`, `ListTemplate`), update, delete and recycle; root folder
  and forms; fields, views and content types; role assignments and inheritance on lists and items;
  webhook subscriptions. `ListInfo` gained template, type, hidden, timestamps, versioning and
  content type flags.
- **SharePoint items**: CAML queries, `count`, `get_changes` with `ChangeQuery`, attachments,
  versions, recycle, update with etag; validated create and update with per-column results;
  formatted values (text, HTML, edit form); content type, parent list, file and folder reads;
  like and unlike, liked-by, comments; restore version by label.
- **Graph sites**: subsites (list, by id, by path), columns, content types, permissions (list,
  read, create, update, delete), `analytics()` and `followed_sites()`. `Site` gained description,
  timestamps, root and site collection information.
- **Graph lists**: create, update (PATCH) and delete; columns; content types (`addCopy`);
  change-notification subscriptions. `List` gained description, timestamps and template/hidden
  flags.
- **Graph items**: delta queries with token resume, versions and `restore_version`, created and
  modified metadata with `IdentitySet`, `drive_item()`, `analytics()`, `activities()`,
  `update_if_match` and `filter_unindexed`.

### Changed

- Internal: SharePoint and Graph list code was split into submodules (`sharepoint/web/`,
  `sharepoint/lists/`, `graph/lists.rs`); the public API only grew.

## [0.0.1-beta] - 2026-10-31

### Added

First release.

- Authentication: username/password, client certificate (PEM and PFX), interactive browser
  (authorization code with PKCE) and device code flows, with per-scope token caching and refresh.
- Fluent Microsoft Graph client: users, sites, lists and list items, drives and files
  (including resumable uploads for large files).
- Fluent SharePoint REST client: web, lists, items, folders and files, with form digest handling.
- Generic authenticated request builder for endpoints without a typed wrapper, with throttling
  retry, structured errors and pagination.
