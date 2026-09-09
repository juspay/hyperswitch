CREATE TABLE resources (
    id VARCHAR(64) NOT NULL PRIMARY KEY,
    resource_type VARCHAR(64) NOT NULL,
    scope VARCHAR(32) NOT NULL,
    scope_id VARCHAR(64) NOT NULL,
    data JSONB NOT NULL DEFAULT '{}'::JSONB,
    encrypted_data BYTEA,
    created_by TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    modified_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE INDEX resources_scope_id_resource_type_index ON resources (scope_id, resource_type);
