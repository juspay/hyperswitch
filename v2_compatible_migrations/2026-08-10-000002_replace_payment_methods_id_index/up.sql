-- Let a payment method id span several rows.
--
-- Applying an account updater card change retires the row holding the old card and inserts a new
-- active row under the same id, so the merchant's identifier never changes. That is impossible
-- while payment_methods_id_index is unique on id alone.
--
-- Two rules replace it, and they are not the same rule:
--
--   * UNIQUE (id, locker_fingerprint_id) stops the same card being stored twice under one id.
--     It does NOT stop two active rows -- two active rows with different fingerprints satisfy it
--     perfectly well. Retired rows carry a NULL fingerprint and are exempt, which is what lets any
--     number of them coexist.
--   * UNIQUE (id) WHERE status = 'active' is what actually holds "one live card per id" up, and is
--     what forces the write order in the apply path: deactivate the old row before inserting the
--     new one.
--
-- status is stored as text, so the predicate is literally the snake_case variant name.
--
-- Built CONCURRENTLY: a plain CREATE UNIQUE INDEX holds a SHARE lock on payment_methods for the
-- whole build, which blocks every write to the table. The trade-off is that a concurrent build is
-- not transactional -- if it finds a duplicate it fails and leaves an INVALID index behind, which
-- costs writes without ever being used. Recovery is manual:
--
--     DROP INDEX CONCURRENTLY payment_methods_id_active_index;   -- then re-run
--
-- The preceding migration exists so that cannot happen.

DROP INDEX CONCURRENTLY IF EXISTS payment_methods_id_index;

CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS payment_methods_id_locker_fingerprint_id_index
    ON payment_methods (id, locker_fingerprint_id);

CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS payment_methods_id_active_index
    ON payment_methods (id) WHERE status = 'active';
