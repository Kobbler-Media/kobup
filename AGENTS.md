# Kobup — engineering conventions

Over-the-air update server for sideloaded Android / Android TV apps.

Machine-specific notes — your paths, your deployment, your local tooling — belong in the gitignored `CLAUDE.local.md` / `AGENTS.local.md`, not here.

## Build

- `make` — full build: fmt + lint + check + build + test. Required before calling anything done.
- `make quick` — fast: fmt + lint + check + build (skip tests)
- `make web` — frontend only
- `make sqlx-prepare` — regenerate the `.sqlx/` offline cache after changing queries
- Warnings are errors: clippy runs with `-D warnings`
- All Rust build targets use `SQLX_OFFLINE=true`, so no database is needed to compile

## Architecture

- **kobup-service** — Rust/Axum backend with SQLite
- **kobup-web** — SvelteKit 5 admin UI (static adapter)
- **kobup-cli** — Rust CLI for CI/dev
- **kobup-android** — Android updater app (Kotlin/Compose)
- **kobup-sdk** — Android SDK library (ContentProvider IPC)

## Backend patterns

- Repository pattern: all DB operations in `crates/kobup-service/src/repositories/`
- Use `sqlx::query!` / `query_as!` / `query_scalar!` macros (compile-time checked)
- After changing any SQL, run `make sqlx-prepare` to regenerate the `.sqlx/` cache
- Dynamic SQL (catalog search, project update) uses runtime `sqlx::query()` — these are the exceptions
- Auth extractors in `routes/mod.rs`: `SessionUser` (any web user), `AuthUser` (admin role), `DeviceAuth` (device), `CiAuth` (CI), `AnyAuth`
- Tokens SHA-256 hashed before storage, never stored plaintext
- Session cookie: `kobup_session` (HttpOnly, SameSite=Lax)
- PIN login is delegated to an external PIN broker (default `https://pin.biboop.com`, override with `KOBUP_PIN_SERVICE_URL` / `-PpinServiceUrl`), namespace `kobup`
- Config from env vars via `Config::from_env()`
- SQLite with WAL mode, foreign keys enabled
- Login rate-limited (5 attempts/minute per IP)
- Tests in `crates/kobup-service/tests/integration.rs`. Bug fixes are test-first — reproduce, then fix.
- Comments explain non-obvious *why*. No docstrings restating what the code does.

## Auth scopes

- `session` — web password login, 30-day expiry; what it can do depends on the user's role:
  `admin` (everything) or `viewer` (browse projects/releases, PIN-authorise their own devices)
- `device` — PIN flow, 365-day expiry, read-only (catalog/download/register)
- `ci` — PIN flow or admin-created API token, 90-day expiry, write (upload/publish/push-refresh)

## Database

- SQLite, structured for easy Postgres migration later
- Migrations in `crates/kobup-service/migrations/`
- UUIDs as TEXT (generated as `Uuid::now_v7().to_string()`)
- Booleans as INTEGER (0/1) — use `"col: bool"` type override in `query_as!`
- Timestamps as TEXT with `datetime('now')`

## Background tasks

- **Artifact retention** (`src/retention.rs`) — prunes releases beyond `MAX_RELEASES_PER_CHANNEL` per project/channel, runs hourly
- **GitHub polling** (`src/github.rs`) — polls GitHub releases for configured projects, interval via `GITHUB_POLL_INTERVAL_SECS`

## Web frontend

- Svelte 5 with SvelteKit, static adapter
- API client at `apps/kobup-web/src/lib/api/client.ts`
- All fetch calls use `credentials: 'include'`
- `VITE_API_BASE_URL` env var for API base

## Android

- `app/google-services.json` and `.kobup.json` are per-developer and gitignored; copy the `.example` files next to them
- The Google Services plugin is applied only if `google-services.json` exists, so builds work without FCM
- Release signing reads `KOBUP_KEYSTORE_PASSWORD` / `KOBUP_KEY_PASSWORD` from `.env` (and `KOBUP_KEYSTORE_PATH` from the environment)
- SDK uses ContentProvider IPC (`com.kobup.app.provider`) for `getUpdateStatus()`
- Intent-based IPC for `openKobup()` (`com.kobup.action.OPEN_APP`)
- All SDK methods are safe when Kobup is not installed
