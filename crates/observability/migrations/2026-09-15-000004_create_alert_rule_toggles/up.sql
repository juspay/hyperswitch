CREATE TABLE alert_rule_toggles (
    rule_id         TEXT PRIMARY KEY,
    is_enabled      BOOLEAN NOT NULL,
    updated_by      TEXT NOT NULL,
    last_updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (rule_id = btrim(rule_id) AND rule_id <> '')
);
