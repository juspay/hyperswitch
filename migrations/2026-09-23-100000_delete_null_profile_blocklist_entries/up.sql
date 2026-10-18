-- Remove blocklist entries that predate profile scoping. Every such entry has already been copied
-- to each business profile of its merchant by the backfill in
-- 2026-08-19-100000_backfill_blocklist_profile_id, so the NULL profile_id rows are no longer needed.
DELETE FROM blocklist WHERE profile_id IS NULL;
