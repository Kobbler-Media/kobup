-- One baseline profile per API band: ART reads a different profile format before API 31.
CREATE TABLE IF NOT EXISTS dex_metadata (
    artifact_id TEXT NOT NULL REFERENCES artifacts(id) ON DELETE CASCADE,
    min_api INTEGER NOT NULL,
    sha256 TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    file_path TEXT NOT NULL,
    PRIMARY KEY (artifact_id, min_api)
);
