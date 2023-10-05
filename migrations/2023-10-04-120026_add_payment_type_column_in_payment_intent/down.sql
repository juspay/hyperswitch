-- This file should undo anything in `up.sql`

ALTER TABLE payment_intent
DROP COLUMN payment_type;

DROP TYPE "PaymentType";
