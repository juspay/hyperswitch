-- Your SQL goes here
-- Your SQL goes here
-- Tables
CREATE TABLE IF NOT EXISTS unified_translations (
    unified_code VARCHAR(255) NOT NULL,
    unified_message VARCHAR(1024) NOT NULL,
    locale VARCHAR(255) NOT NULL DEFAULT 'en',
    translation VARCHAR(1024) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP,
    last_modified TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP,
    PRIMARY KEY (unified_code,unified_message,locale)
);