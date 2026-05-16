# AGENTS.md

## Scope

These instructions apply to the whole repo.

## Project

`apstr` is a small Rust 2024 web app. It uses:

- `axum` for HTTP routes
- `maud` for HTML views
- `seekwel` / `rusqlite` with local SQLite
- static assets in `assets/`

## Commands

- Run locally: `./bin/dev`
- Format: `cargo fmt`
- Check: `cargo check`
- Test: `cargo test`

## Conventions

- Prefer small, direct changes.
- Do not add third-party dependencies unless explicitly requested.
- Do not wrap the database/model layer in extra abstraction; use the existing `seekwel` / `rusqlite` patterns directly.
- Put route handlers in `src/controllers/`.
- Put HTML rendering in `src/views/`.
- Put shared helpers in `src/helpers/`.
- Put external/service code in `src/library/`.
- Put persisted types in `src/models/`.

## Local/private files

- Treat `apstr.sqlite*`, `pvt/`, and `.envrc` as local/private state.
- Do not depend on them for correctness.
- Do not commit generated database files.

## Notes

- Assets are served from `assets/` in debug builds.
- Release builds embed assets via `build.rs`.
- Schema changes are applied at startup, so review model changes carefully.
