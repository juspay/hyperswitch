CREATE TABLE alert_rules (
    id                VARCHAR(64)  PRIMARY KEY,
    name              VARCHAR(255) NOT NULL,
    config            JSONB        NOT NULL,
    status            VARCHAR(32)  NOT NULL DEFAULT 'active',
    next_run_at       TIMESTAMP    NOT NULL DEFAULT date_trunc('second', CURRENT_TIMESTAMP),
    last_evaluated_at TIMESTAMP,
    created_at        TIMESTAMP    NOT NULL DEFAULT date_trunc('second', CURRENT_TIMESTAMP),
    last_updated_at   TIMESTAMP    NOT NULL DEFAULT date_trunc('second', CURRENT_TIMESTAMP)
);

CREATE INDEX idx_alert_rules_due ON alert_rules (next_run_at) WHERE status = 'active';
