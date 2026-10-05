-- Apply before deploying code that reads or writes execution_kind.
CREATE TYPE "PayoutExecutionKind" AS ENUM ('normal', 'external_vault_proxy');

ALTER TABLE payout_attempt
    ADD COLUMN execution_kind "PayoutExecutionKind" NOT NULL DEFAULT 'normal';
