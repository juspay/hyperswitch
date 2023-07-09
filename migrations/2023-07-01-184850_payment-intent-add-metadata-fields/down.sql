-- This file should undo anything in `up.sql`
ALTER TABLE payment_intent 
DROP COLUMN allowed_payment_method_types,
DROP COLUMN connector_metadata,
DROP COLUMN feature_metadata;
