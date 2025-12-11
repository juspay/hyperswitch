-- This file should undo anything in `up.sql`
ALTER TABLE gateway_status_map
    DROP COLUMN IF EXISTS standardised_code,
    DROP COLUMN IF EXISTS description,
    DROP COLUMN IF EXISTS user_guidance_message;
