CREATE TABLE alert_lifecycle_events (
    alert_key       VARCHAR(32) PRIMARY KEY,
    detector        TEXT NOT NULL,
    merchant_id     TEXT NOT NULL,
    profile_id      TEXT NOT NULL DEFAULT '',
    state           TEXT NOT NULL,
    first_seen      TIMESTAMP NOT NULL,
    last_seen       TIMESTAMP NOT NULL,
    recovered_at    TIMESTAMP NOT NULL,
    runs            BIGINT NOT NULL,
    severity        TEXT NOT NULL DEFAULT '',
    sr              DOUBLE PRECISION NOT NULL,
    failed          BIGINT NOT NULL,
    total           BIGINT NOT NULL,
    connector       TEXT NOT NULL DEFAULT '',
    notified_at     TIMESTAMP NOT NULL,
    ts_slack        TEXT NOT NULL DEFAULT '',
    sent            BOOLEAN NOT NULL,
    last_updated_at TIMESTAMP NOT NULL,
    CHECK (char_length(alert_key) = 32),
    CHECK (state IN ('firing', 'recovered')),
    CHECK (runs >= 0),
    CHECK (failed >= 0),
    CHECK (total >= 0)
);

CREATE INDEX alert_lifecycle_events_window_idx
    ON alert_lifecycle_events (last_seen, first_seen);
CREATE INDEX alert_lifecycle_events_updated_idx
    ON alert_lifecycle_events (last_updated_at);
