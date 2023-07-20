-- This file should undo anything in `up.sql`
ALTER TABLE payment_intent ADD COLUMN udf JSONB;
