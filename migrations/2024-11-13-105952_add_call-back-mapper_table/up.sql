-- Your SQL goes here
CREATE TABLE IF NOT EXISTS callback_mapper (
    id VARCHAR(128) NOT NULL,
    type VARCHAR(64) NOT NULL,
    data JSONB NOT NULL,
    created_at TIMESTAMP NOT NULL,
    last_modified_at TIMESTAMP NOT NULL,
    PRIMARY KEY (id, type)
);