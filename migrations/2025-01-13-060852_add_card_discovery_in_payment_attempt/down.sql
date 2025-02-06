-- This file should undo anything in `up.sql`
ALTER TABLE payment_attempt DROP COLUMN IF EXISTS card_discovery;

DROP TYPE IF EXISTS "CardDiscovery";
