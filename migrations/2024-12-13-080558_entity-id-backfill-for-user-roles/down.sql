-- This file should undo anything in `up.sql`
UPDATE user_roles SET entity_id = NULL WHERE version = 'v1';