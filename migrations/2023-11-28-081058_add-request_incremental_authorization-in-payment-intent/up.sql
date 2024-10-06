-- Your SQL goes here
CREATE TYPE "RequestIncrementalAuthorization" AS ENUM ('true', 'false', 'default');
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS request_incremental_authorization "RequestIncrementalAuthorization" NOT NULL DEFAULT 'false'::"RequestIncrementalAuthorization";
