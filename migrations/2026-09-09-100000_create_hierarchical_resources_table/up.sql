CREATE TABLE hierarchical_resources (
    id VARCHAR(64) NOT NULL PRIMARY KEY,
    resource_type VARCHAR(64) NOT NULL,
    scope VARCHAR(32) NOT NULL,
    scope_id VARCHAR(64) NOT NULL,
    data JSONB NOT NULL,
    encrypted_data BYTEA,
    created_by TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    modified_at TIMESTAMP NOT NULL
);

CREATE INDEX hierarchical_resources_scope_id_resource_type_index ON hierarchical_resources (scope_id, resource_type);
