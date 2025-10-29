-- Your SQL goes here
CREATE TABLE hyperswitch_ai_interaction (
    id VARCHAR(64) NOT NULL,
    session_id VARCHAR(64),
    user_id VARCHAR(64),
    merchant_id VARCHAR(64),
    profile_id VARCHAR(64),
    org_id VARCHAR(64),
    role_id VARCHAR(64),
    user_query BYTEA,
    response BYTEA,
    database_query TEXT,
    interaction_status VARCHAR(64),
    created_at TIMESTAMP NOT NULL,
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

-- Create a default partition
CREATE TABLE hyperswitch_ai_interaction_default
    PARTITION OF hyperswitch_ai_interaction DEFAULT;

