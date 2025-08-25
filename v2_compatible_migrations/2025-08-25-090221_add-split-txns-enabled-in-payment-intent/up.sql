-- Your SQL goes here
CREATE TYPE "SplitTxnsEnabled" AS ENUM ('true', 'false', 'default');
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS split_txns_enabled "SplitTxnsEnabled" DEFAULT 'false'::"SplitTxnsEnabled";
