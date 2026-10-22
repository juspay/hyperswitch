CREATE TABLE alert_rules (
    id                   VARCHAR(64)  PRIMARY KEY,
    name                 VARCHAR(255) NOT NULL,
    status               VARCHAR(32)  NOT NULL,
    alerts_configuration JSONB        NOT NULL,
    next_run_at          TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at           TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_updated_at      TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX alert_rules_due_idx ON alert_rules (next_run_at) WHERE status = 'active';
