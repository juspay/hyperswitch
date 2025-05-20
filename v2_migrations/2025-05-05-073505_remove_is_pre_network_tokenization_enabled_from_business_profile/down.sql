-- This file should undo anything in `up.sql`
ALTER TABLE business_profile ADD COLUMN IF NOT EXISTS is_pre_network_tokenization_enabled BOOLEAN;