-- Apply this additive migration before deploying code that selects/inserts execution_kind.
-- DEFAULT 'normal' covers existing rows and inserts from the previous application version.
-- The new application also needs #[serde(default)] to read pre-deploy Redis/drainer records.
CREATE TYPE "PayoutExecutionKind" AS ENUM ('normal', 'external_vault_proxy');

ALTER TABLE payout_attempt
    ADD COLUMN execution_kind "PayoutExecutionKind" NOT NULL DEFAULT 'normal';
