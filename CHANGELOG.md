# Changelog

All notable changes to this project are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project
uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

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
