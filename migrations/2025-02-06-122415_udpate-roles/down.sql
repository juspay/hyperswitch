-- This file should undo anything in `up.sql`
ALTER TABLE roles ALTER COLUMN merchant_id SET NOT NULL;