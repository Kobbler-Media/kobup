# Contributing to Kobup

Pull requests are welcome — bug fixes, new release-channel behaviour, UI work, Android improvements, docs. Open an issue first only if you want to sanity-check a design direction before writing code.

## Licence

Kobup is licensed under the **GNU Affero General Public License v3.0** ([LICENSE](LICENSE)). By submitting a contribution you agree that it is licensed under AGPL-3.0 on the same terms, and that you have the right to submit it.

## Getting set up

```bash
git clone https://github.com/Kobbler-Media/kobup.git
cd kobup
cp .env.example .env
make install        # npm ci for apps/kobup-web
cargo run -p kobup-service
```

You need a recent stable Rust toolchain and Node.js 24+. Docker is optional but makes the full stack a one-liner (`docker compose up --build`). Nothing depends on a sibling checkout or a private registry.

For the Android side you also need Android Studio (or a JDK plus the Android SDK), and you must supply your own `apps/kobup-android/app/google-services.json` and `apps/kobup-android/.kobup.json` — both are gitignored and have `.example` templates next to them. See the README for details.

## Build and test

```bash
make          # fmt + lint + check + build + test — run this before opening a PR
make quick    # everything except tests, for the inner loop
make web      # frontend only
```

`make` must be clean. Warnings are errors (`clippy -D warnings`).

| Target | What it does |
|---|---|
| `make test` | `cargo test` — the suite builds its own temporary SQLite databases, no services needed |
| `make sqlx-prepare` | Regenerates the `.sqlx/` offline query cache; run it after changing any SQL |
| `make sqlx-check` | Verifies the committed `.sqlx/` cache matches the code |
| `make docker-push` | Maintainers only; needs push rights on the release registry |

`sqlx-prepare` and `sqlx-check` need `sqlx-cli` with the SQLite driver:

```bash
cargo install sqlx-cli --no-default-features --features postgres,sqlite,rustls
```

The targets check for this and tell you if it's missing. Everything else runs with `SQLX_OFFLINE=true` against the committed cache, so no database is required to build.

The web targets are skipped with a notice if `apps/kobup-web/node_modules` is absent — run `make install` first if you're touching the frontend.

## Bug fixes are test-first

If you are fixing a bug or changing behaviour, **write a failing test first**:

1. Add a test to `crates/kobup-service/tests/integration.rs` that reproduces the bug. Run it and watch it fail.
2. Fix the bug.
3. Run it again and watch it pass.

Prefer integration tests over unit tests — the suite spins up the real router against a temporary database, so most behaviour is testable end to end. Cover every entry point into the broken logic, not just the one that surfaced the report: the same repository call is often reachable from an admin route, a CI route and a public route.

## Code conventions

- **Repository pattern.** All database access lives in `crates/kobup-service/src/repositories/`. Routes call repositories; routes do not write SQL.
- **Compile-time-checked SQL.** Use `sqlx::query!` / `query_as!` / `query_scalar!`. Dynamic SQL (catalog search, project update) uses runtime `sqlx::query()` — those are the deliberate exceptions. Run `make sqlx-prepare` after any SQL change and commit the `.sqlx/` diff.
- **Auth extractors** live in `routes/mod.rs`: `AuthUser` (admin), `DeviceAuth` (device), `CiAuth` (CI), `AnyAuth`. Pick the narrowest one that works.
- **Never store a token in plaintext.** Tokens are SHA-256 hashed before they touch the database.
- **SQLite conventions**: UUIDs as TEXT (`Uuid::now_v7().to_string()`), booleans as INTEGER with a `"col: bool"` type override in `query_as!`, timestamps as TEXT via `datetime('now')`. The schema is kept Postgres-portable on purpose.
- **Config** comes from environment variables via `Config::from_env()`. Add new variables there, to `.env.example`, and to the README table.
- **Frontend**: Svelte 5 with the static adapter. All fetches go through `apps/kobup-web/src/lib/api/client.ts` and use `credentials: 'include'`.
- **Comments explain why, not what.** The codebase is deliberately light on comments; if code needs a comment to be readable, prefer renaming or refactoring it.
- **No compatibility shims or commented-out code.** Delete what you replace.

## Pull requests

- One logical change per PR, with a description of the behaviour before and after.
- `make` clean, tests included for behaviour changes.
- Don't commit `.env`, `google-services.json`, `.kobup.json`, keystores, APKs, or anything from `data/`.

## Machine-specific notes

Anything about *your* machine, your deployment, your paths or your agent tooling belongs in the gitignored `CLAUDE.local.md` / `AGENTS.local.md`, not in `CLAUDE.md`.
