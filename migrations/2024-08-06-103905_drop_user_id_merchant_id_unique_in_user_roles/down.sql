-- This file should undo anything in `up.sql`
ALTER TABLE user_roles ADD CONSTRAINT user_merchant_unique UNIQUE (user_id, merchant_id);
