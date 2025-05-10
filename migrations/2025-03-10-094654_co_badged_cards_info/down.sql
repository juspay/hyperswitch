-- This file should undo anything in `up.sql`
DROP INDEX IF EXISTS co_badged_cards_card_bin_min_card_bin_max_index;

DROP TABLE IF EXISTS co_badged_cards_info;

DROP TYPE IF EXISTS "CardType";

DROP TYPE IF EXISTS "PanOrToken";