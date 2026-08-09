# Changelog

All notable changes to the Sova workspace are recorded here.
Versions refer to the published crates on [crates.io](https://crates.io/crates/sova).

## 0.1.9 — 2026-08-09

- Docs: CSRF field `csrf`, no double-install after `App::web()`
- Docs-gen: prefer plugin id matching page slug (`meta` not `sitemap`)
- `Notifications::ws_path` requires installed `ws`
- `cargo sovax generate plugin` uses crates.io `sova-core` (no monorepo path)
- Facade `sova` 0.1.9 (auth 0.1.5, notifications 0.1.3, cargo-sovax 0.1.6)

## 0.1.8 — 2026-08-09

- Facade crates.io README: use root README (remove conflicting stub)

## 0.1.7 — 2026-08-09

Package / docs hygiene across the monorepo:

- Shared `[workspace.package]` (`authors`, `license`, `repository`, `homepage`, `edition`)
- `README.md` + `LICENSE` in every published crate and plugin (crates.io badges)
- Index READMEs under `crates/` and `plugins/`
- Patch bump of all published packages for metadata packaging
- Break publish cycle: auth/notification test helpers moved to `sova_auth::testing` / `sova_notifications::testing`; `sova-testing` stays core+db only

## 0.1.6 — 2026-08-09

DX simplification:

- Unique `MigrationName` across plugins / examples / `cargo-sovax` codegen
- `TestClient::boot` / `tracked` run startup and accept `Into<App>`
- `Db::seed` accepts `Into<Error>` (facade `AppError` works)
- Facade `auth` without mail by default; use `auth-mail` for verify/reset
- `meta` no longer enables OpenAPI by default (`meta-openapi`)
- `Error::bad_request`; Fortify / session / testing docs updates

## 0.1.5 — earlier

HN-style example, Fortify Registration-only path, release smoke scripts, and related auth/db fixes.

See git history for older workspace commits.
