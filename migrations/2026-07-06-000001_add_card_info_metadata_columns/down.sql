ALTER TABLE cards_info
DROP COLUMN IF EXISTS funding_source,
DROP COLUMN IF EXISTS card_iin_type,
DROP COLUMN IF EXISTS virtual_card,
DROP COLUMN IF EXISTS gambling_blocked,
DROP COLUMN IF EXISTS co_badged_card_networks;
