ALTER TABLE cards_info
DROP COLUMN IF EXISTS funding_source,
DROP COLUMN IF EXISTS pan_or_token,
DROP COLUMN IF EXISTS virtual_card,
DROP COLUMN IF EXISTS gambling_blocked;
