-- Users
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY NOT NULL,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'admin',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Auth tokens (web session + device, SHA256 hashed)
CREATE TABLE IF NOT EXISTS auth_tokens (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL,
    scope TEXT NOT NULL DEFAULT 'admin',
    expires_at TEXT,
    revoked INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_auth_tokens_hash ON auth_tokens(token_hash);

-- Projects
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    package_name TEXT NOT NULL DEFAULT '',
    visibility TEXT NOT NULL DEFAULT 'public',
    default_channel TEXT NOT NULL DEFAULT 'stable',
    source_type TEXT NOT NULL DEFAULT 'manual',
    source_config_json TEXT,
    allowed_signing_fingerprints_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- API tokens (CI tokens with names, project scoping)
CREATE TABLE IF NOT EXISTS api_tokens (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    token_hash TEXT NOT NULL,
    scopes_json TEXT NOT NULL DEFAULT '["ci"]',
    last_used_at TEXT,
    revoked_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_api_tokens_hash ON api_tokens(token_hash);

-- Channels
CREATE TABLE IF NOT EXISTS channels (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(project_id, name)
);

-- Releases
CREATE TABLE IF NOT EXISTS releases (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    channel_id TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    version_code INTEGER NOT NULL,
    version_name TEXT NOT NULL,
    release_notes TEXT,
    source_type TEXT NOT NULL DEFAULT 'upload',
    source_ref TEXT,
    published_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(project_id, channel_id, version_code)
);

-- Artifacts
CREATE TABLE IF NOT EXISTS artifacts (
    id TEXT PRIMARY KEY NOT NULL,
    release_id TEXT NOT NULL REFERENCES releases(id) ON DELETE CASCADE,
    file_name TEXT NOT NULL,
    package_name TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    min_sdk INTEGER,
    target_sdk INTEGER,
    version_code INTEGER NOT NULL,
    version_name TEXT NOT NULL,
    supported_abis_json TEXT NOT NULL DEFAULT '[]',
    is_universal INTEGER NOT NULL DEFAULT 1,
    signing_fingerprint TEXT,
    file_path_or_object_key TEXT NOT NULL,
    mime_type TEXT NOT NULL DEFAULT 'application/vnd.android.package-archive',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Devices
CREATE TABLE IF NOT EXISTS devices (
    id TEXT PRIMARY KEY NOT NULL,
    device_name TEXT NOT NULL,
    platform TEXT NOT NULL DEFAULT 'android',
    sdk_int INTEGER,
    abis_json TEXT NOT NULL DEFAULT '[]',
    is_tv INTEGER NOT NULL DEFAULT 0,
    kobup_version TEXT,
    app_version_code INTEGER,
    last_seen_at TEXT NOT NULL DEFAULT (datetime('now')),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Device push tokens
CREATE TABLE IF NOT EXISTS device_push_tokens (
    id TEXT PRIMARY KEY NOT NULL,
    device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    provider TEXT NOT NULL DEFAULT 'fcm',
    token TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_device_push_tokens_device ON device_push_tokens(device_id, provider);

-- Install events
CREATE TABLE IF NOT EXISTS install_events (
    id TEXT PRIMARY KEY NOT NULL,
    device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    package_name TEXT NOT NULL,
    from_version_code INTEGER,
    to_version_code INTEGER,
    status TEXT NOT NULL,
    message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
