-- Drain column-dependent code and reconcile proxy attempts before rollback.
ALTER TABLE payout_attempt DROP COLUMN execution_kind;
