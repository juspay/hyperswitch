CREATE TABLE alert_rule_configurations (
    id              VARCHAR(64)  PRIMARY KEY,
    name            VARCHAR(255) NOT NULL UNIQUE,
    is_enabled      BOOLEAN      NOT NULL DEFAULT TRUE,
    config          JSONB        NOT NULL,
    next_run_at     TIMESTAMP    NOT NULL DEFAULT date_trunc('second', CURRENT_TIMESTAMP),
    created_at      TIMESTAMP    NOT NULL DEFAULT date_trunc('second', CURRENT_TIMESTAMP),
    last_updated_at TIMESTAMP    NOT NULL DEFAULT date_trunc('second', CURRENT_TIMESTAMP)
);

CREATE INDEX idx_alert_rule_configurations_due
    ON alert_rule_configurations (next_run_at) WHERE is_enabled;
