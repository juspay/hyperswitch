-- Stop/drain code that reads or writes execution_kind before removing the column.
-- After proxy execution is enabled, retain the marker until proxy attempts are reconciled;
-- rolling back to a normal-only binary would otherwise lose their execution-mode protection.
ALTER TABLE payout_attempt DROP COLUMN execution_kind;

DROP TYPE "PayoutExecutionKind";
