-- This file should undo anything in `up.sql`
UPDATE user_roles SET entity_type = 'internal' where role_id like 'internal%' and version = 'v2';
