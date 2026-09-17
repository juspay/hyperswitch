-- The singleton row serializes cap checks with creates/reactivations. PostgreSQL's
-- default READ COMMITTED isolation then makes the count following the row lock see
-- every previously committed blacklist write.
CREATE TABLE alert_blacklist_write_lock (
    lock_key SMALLINT PRIMARY KEY
);

INSERT INTO alert_blacklist_write_lock (lock_key) VALUES (1);

CREATE TABLE alert_blacklist (
    rule_id         TEXT NOT NULL,
    merchant_id     TEXT NOT NULL,
    profile_id      TEXT NOT NULL DEFAULT '',
    reason          TEXT NOT NULL DEFAULT '',
    created_by      TEXT NOT NULL,
    last_updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    is_deleted      BOOLEAN NOT NULL DEFAULT FALSE,

    PRIMARY KEY (rule_id, merchant_id, profile_id)
);
