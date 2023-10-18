-- This file should undo anything in `up.sql`
ALTER TABLE payment_intent
DROP COLUMN IF EXISTS payment_confirm_source;

DROP TYPE "PaymentSource";