# Kobup

Kobup is a small self-hosted over-the-air update server for Android and Android TV apps: you upload an APK, it versions it into a release channel, and paired devices pick it up and install it. It exists for sideloaded apps that can't or don't want to go through the Play Store.

Licensed under [AGPL-3.0](LICENSE). Contributions welcome — see [CONTRIBUTING.md](CONTRIBUTING.md).

## What's in the box

| Component | What it is |
|---|---|
| `crates/kobup-service` | Rust/Axum API server, SQLite storage, APK artifacts on disk |
| `crates/kobup-cli` | `kobup` CLI for CI pipelines and local publishing |
| `apps/kobup-web` | SvelteKit 5 admin dashboard (static build, nginx in Docker) |
| `apps/kobup-android` | The updater app itself (Kotlin/Compose, phone + TV) |
| `apps/kobup-sdk` | Android library so a host app can query update status over IPC |
| `apps/kobup-sample-host` | Minimal app demonstrating the SDK |

## How it relates to Kobbler

[Kobbler](https://github.com/Kobbler-Media/kobbler) is a self-hosted media and ebook server; its Android app (`kobapp`) is sideloaded onto TVs and set-top boxes, and it uses Kobup for over-the-air updates. That dependency is one-directional and configuration-only: the Kobbler app points at a Kobup server URL. Kobup itself knows nothing about Kobbler and is useful for any sideloaded Android app.

## Prerequisites

- Docker with Compose v2 — that's all you need for the quickstart.
- For local (non-Docker) development: a recent stable Rust toolchain and Node.js 24+.
- For the Android apps: Android Studio, or a JDK plus the Android SDK.

There are no sibling-checkout or private-registry requirements; the whole stack builds from this repo.

## Quickstart

```bash
git clone https://github.com/Kobbler-Media/kobup.git
cd kobup
cp .env.example .env        # edit it — at minimum set ADMIN_BOOTSTRAP_PASSWORD
docker compose up -d --build
```

- Web UI: <http://localhost:5175>
- API: <http://localhost:8080>

The admin user is created on first boot from `ADMIN_EMAIL` / `ADMIN_BOOTSTRAP_PASSWORD`. If you left the password blank, a random one is printed to the API logs (`docker compose logs api`). The SQLite database and uploaded APKs live in the `kobup_data` named volume; set `KOBUP_DATA` to a host path if you'd rather bind-mount.

### First run, end to end

1. **Create a project** in the web UI: a display name, a URL slug, and the Android package name.
2. **Upload an APK** on the project page, then publish the release to put it in the catalog.
3. **Set up the CLI** for CI or scripted publishing:
   ```bash
   cargo install --path crates/kobup-cli
   kobup login                                  # prints a PIN; enter it under "Login CLI" in the web UI
   kobup init --slug my-app                     # writes .kobup.json in the current directory
   kobup push app-release.apk                   # upload + release + publish + notify
   ```
4. **Pair a device**: install the Kobup Android app, which displays a 6-digit PIN. Enter that PIN in the web UI. The device then pulls the catalog and installs updates; FCM push (if configured) makes it immediate rather than on the next poll.

## Local development

### API

```bash
cp .env.example .env
cargo run -p kobup-service      # http://localhost:8080
```

### Web UI

```bash
cd apps/kobup-web
npm install
VITE_API_BASE_URL=http://localhost:8080 npm run dev
```

### Hot-reload Docker stack

```bash
docker compose -f docker-compose.dev.yml up --build
```

### Android

Open `apps/kobup-android` in Android Studio. Two things are **not** in the repo and you must supply them yourself:

- **`app/google-services.json`** — Firebase config for push notifications. Copy `app/google-services.json.example` and fill it in from your own Firebase project (Firebase console → Project settings → Your apps → Android app → *Download google-services.json*). The build does **not** fail without it: `app/build.gradle.kts` applies the Google Services plugin only when the file exists, so the app compiles and runs with FCM push simply disabled.
- **`.kobup.json`** — deployment target for `./gradlew :app:publishToKobup`. Copy `.kobup.json.example` to `.kobup.json` in `apps/kobup-android/` and set `server` to your Kobup instance. This file is read from a fixed path (`<project root>/.kobup.json`) and only by the `publishToKobup` task, so a plain build or `assembleDebug` does not need it. Publishing also needs `KOBUP_TOKEN` (a CI token created in the web admin) in the environment or in `apps/kobup-android/.env`, and a release keystore — see `KOBUP_KEYSTORE_PATH` in `.env.example`.

Both files are gitignored. `scripts/setup-android.sh` generates the Gradle wrappers if they are missing.

## Build and test

```bash
make          # fmt + lint + check + build + test
make quick    # same, skipping tests
make web      # frontend only
make install  # npm ci for apps/kobup-web
```

See [CONTRIBUTING.md](CONTRIBUTING.md).

## Configuration

| Variable | Default | Required | Description |
|---|---|---|---|
| `DATABASE_URL` | `sqlite:data/kobup.db?mode=rwc` | No | SQLite connection string |
| `BIND_ADDR` | `0.0.0.0:8080` | No | Listen address |
| `STORAGE_ROOT` | `data` | No | Directory for APK artifacts |
| `KOBUP_BASE_URL` | `http://localhost:8080` | In production | Public base URL, embedded in download links and pairing payloads |
| `KOBUP_WEB_ORIGINS` | *(empty = allow all)* | In production | Comma-separated CORS origins |
| `ADMIN_EMAIL` | `admin@localhost` | No | Bootstrap admin email |
| `ADMIN_BOOTSTRAP_PASSWORD` | *(generated)* | Recommended | Bootstrap admin password; first boot only |
| `KOBUP_DATA` | `kobup_data` | No | Named volume or host path for `/data` (Docker only) |
| `KOBUP_API_PORT` / `KOBUP_WEB_PORT` | `8080` / `5175` | No | Host ports (Docker only) |
| `FCM_SERVICE_ACCOUNT_JSON` | *(none)* | For push | In-container path to a Firebase service account JSON |
| `KOBUP_GITHUB_TOKEN` | *(none)* | For GitHub ingest | PAT for pulling releases from GitHub Releases |
| `GITHUB_POLL_INTERVAL_SECS` | `3600` | No | GitHub poll interval |
| `MAX_RELEASES_PER_CHANNEL` | `10` | No | Retention per project/channel; older releases pruned hourly |
| `RUST_LOG` | `info` | No | Tracing filter |

`.env.example` is the authoritative list and documents each one inline.

## CLI reference

```
kobup login                      Print a PIN; enter it under "Login CLI" in the web UI to authenticate
kobup init --slug <slug>         Link the current directory to a project (.kobup.json)
kobup push <file.apk>            Upload, create release, publish, notify devices
kobup push <file.apk> --channel beta
kobup push-refresh               Trigger an FCM push without uploading
kobup status                     Server connectivity and auth status
kobup doctor                     Validate local config and credentials
```

## Architecture

```
                +-------------+
                |  kobup-web  |  SvelteKit admin UI
                |  (nginx)    |  static build
                +------+------+
                       |
                       v
+----------+   +------+------+   +-----------+
| kobup-cli| > |kobup-service| > |  SQLite   |
| (CI/dev) |   |   (Axum)    |   +-----------+
+----------+   +------+------+   +-----------+
                       ^          | artifacts |
                       |          |  (disk)   |
                +------+------+   +-----------+
                |kobup-android|
                |  (Kotlin)   |
                +-------------+
```

### Auth model

| Scope | How you get it | Expiry | Can do |
|---|---|---|---|
| `admin` | Email + password login in the web UI | 30 days | Everything |
| `device` | PIN pairing from the Android app | 365 days | Read catalog, download, register |
| `ci` | PIN flow or an admin-created API token | 90 days | Upload, publish, push-refresh |

Tokens are SHA-256 hashed before storage. The admin session cookie is `kobup_session` (HttpOnly, SameSite=Lax). Login is rate-limited to 5 attempts per minute per IP.

## API overview

**Public (no auth)** — `GET /health`, `GET /api/v1/catalog`, `GET /api/v1/projects/{slug}/latest`, `GET /api/v1/projects/{id}/icon`, `GET /api/v1/packages/{package}/latest`, `GET /api/v1/artifacts/{id}/download`, `GET /api/v1/artifacts/{id}/dm/{minApi}`, `POST /api/v1/devices/register`

**Admin (bearer token or session cookie)** — `POST /api/v1/admin/login`, `GET/POST /api/v1/admin/projects`, `POST /api/v1/admin/projects/{id}/upload`, `POST /api/v1/admin/projects/{id}/icon`, `POST /api/v1/admin/releases/{id}/publish`, `POST /api/v1/admin/push-refresh`

**CI (API token)** — `POST /api/v1/ci/projects/{slug}/release`, `POST /api/v1/ci/projects/{slug}/push-refresh`

**Baseline profiles** — an upload may carry the APK's dex metadata as `dm_<minApi>` fields, one per API band (AGP writes them under `baselineProfiles/` with an `output-metadata.json` beside the APK; `kobup push` and `publishToKobup` send them automatically). `latest` returns the band matching the `sdkInt` query as `dexMetadata`, and the Android app stages it with the APK so ART compiles the app at install time instead of leaving it on the JIT.

## Production notes

- Run behind a reverse proxy that terminates TLS.
- Set `KOBUP_BASE_URL` to the public URL — devices use it to fetch APKs.
- Set `KOBUP_WEB_ORIGINS` to your web UI origin; leaving it empty allows all origins.
- Always set `ADMIN_BOOTSTRAP_PASSWORD` explicitly.
- For push: create a Firebase project, enable Cloud Messaging, download a service account key, mount it into the container and point `FCM_SERVICE_ACCOUNT_JSON` at the in-container path.

## Known limitations

- The device endpoints (`/api/v1/devices/register`, `push-token`, `heartbeat`, `install-event`)
  take no credential at all — any client that can reach the API can register a device, attach
  a push token to an existing device ID, or post a heartbeat/install event for one. This matches
  the Android app's own pairing-free bootstrap, but it means these calls are not currently
  authenticated the way admin/CI routes are. Fixing it needs a device-credential protocol shared
  between this service and the Android client; tracked as a follow-up rather than fixed here.

## Troubleshooting

| Problem | Fix |
|---|---|
| Can't connect to the database | Check `/data` is writable; in Docker verify the `kobup_data` volume exists |
| Login returns 401 | `ADMIN_BOOTSTRAP_PASSWORD` only applies on first boot — delete the database to re-bootstrap |
| Upload fails | `STORAGE_ROOT` must exist and be writable (`/data` in Docker) |
| FCM not working | Verify `FCM_SERVICE_ACCOUNT_JSON` points at a valid service account JSON inside the container; check API logs |
| Web UI can't reach the API | Locally, set `VITE_API_BASE_URL=http://localhost:8080`; in Docker, the API must pass its health check first |
| CORS errors | Set `KOBUP_WEB_ORIGINS` to the exact origin including protocol and port |
