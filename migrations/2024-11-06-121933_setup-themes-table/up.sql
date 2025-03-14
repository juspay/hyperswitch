-- Your SQL goes here
CREATE TABLE IF NOT EXISTS themes (
    theme_id VARCHAR(64) PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL,
    org_id VARCHAR(64),
    merchant_id VARCHAR(64),
    profile_id VARCHAR(64),
    created_at TIMESTAMP NOT NULL,
    last_modified_at TIMESTAMP NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS themes_index ON themes (
    tenant_id,
    COALESCE(org_id, '0'),
    COALESCE(merchant_id, '0'),
    COALESCE(profile_id, '0')
);
