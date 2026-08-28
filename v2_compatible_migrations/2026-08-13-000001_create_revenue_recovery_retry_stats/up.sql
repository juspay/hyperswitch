CREATE TABLE IF NOT EXISTS revenue_recovery_retry_stats (
    cluster_key  TEXT  NOT NULL PRIMARY KEY,
    stats JSONB NOT NULL
);
