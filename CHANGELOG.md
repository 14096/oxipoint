# Changelog

All notable changes to this project are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project
uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

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

- **`rsp::fields`**: typed list item column values (`Fields`, `FieldValue`) that serialize for
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
