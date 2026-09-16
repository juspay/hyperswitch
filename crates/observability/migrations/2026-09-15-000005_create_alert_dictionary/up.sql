CREATE TABLE alert_dictionary (
    name            TEXT NOT NULL,
    key_            TEXT NOT NULL,
    product         TEXT NOT NULL DEFAULT '[]',
    values_         TEXT NOT NULL DEFAULT '[]',
    metadata        TEXT NOT NULL DEFAULT '{}',
    updated_by      TEXT NOT NULL,
    last_updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (name, key_),
    CHECK (name = btrim(name) AND name <> ''),
    CHECK (key_ = btrim(key_) AND key_ <> '')
);
