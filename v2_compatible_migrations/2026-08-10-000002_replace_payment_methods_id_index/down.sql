-- Restore the single unique index on id.
--
-- This fails once any card change has been applied, and that is correct: a second row under an id
-- is data the old index cannot represent. Collapse those rows first if the rollback is intended.
--
-- The failed concurrent build leaves payment_methods_id_index behind as INVALID -- it costs writes
-- and is never used by the planner. Clear it before retrying:
--
--     DROP INDEX CONCURRENTLY payment_methods_id_index;
DROP INDEX CONCURRENTLY IF EXISTS payment_methods_id_active_index;

DROP INDEX CONCURRENTLY IF EXISTS payment_methods_id_locker_fingerprint_id_index;

CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS payment_methods_id_index ON payment_methods (id);
