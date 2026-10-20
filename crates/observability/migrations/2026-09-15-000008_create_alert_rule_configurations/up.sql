CREATE TABLE alert_rule_configurations (
    id                VARCHAR(64)  PRIMARY KEY,
    name              VARCHAR(255) NOT NULL UNIQUE,
    classification    VARCHAR(64)  NOT NULL,
    datasource        VARCHAR(64)  NOT NULL,
    query             JSONB        NOT NULL,
    thresholds        JSONB        NOT NULL,
    interval_seconds  INTEGER      NOT NULL,
    is_enabled        BOOLEAN      NOT NULL DEFAULT TRUE,
    next_run_at       TIMESTAMP    NOT NULL DEFAULT date_trunc('second', CURRENT_TIMESTAMP),
    last_evaluated_at TIMESTAMP,
    created_at        TIMESTAMP    NOT NULL DEFAULT date_trunc('second', CURRENT_TIMESTAMP),
    last_updated_at   TIMESTAMP    NOT NULL DEFAULT date_trunc('second', CURRENT_TIMESTAMP)
);

CREATE INDEX idx_alert_rule_configurations_due
    ON alert_rule_configurations (next_run_at) WHERE is_enabled;
