-- This file should undo anything in `up.sql`

ALTER TABLE business_profile
DROP COLUMN guest_user_card_blocking_threshold;