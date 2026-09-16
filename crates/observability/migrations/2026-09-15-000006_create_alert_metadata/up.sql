CREATE TABLE alert_metadata (
    id              TEXT PRIMARY KEY,
    metadata        TEXT NOT NULL DEFAULT '{}',
    snooze          TEXT NOT NULL DEFAULT '',
    updated_by      TEXT NOT NULL,
    last_updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (id = btrim(id) AND id <> '')
);
