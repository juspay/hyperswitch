-- This file should undo anything in `up.sql`
ALTER TABLE payment_intent
ADD id SERIAL;

ALTER TABLE payment_attempt
ADD id SERIAL;
