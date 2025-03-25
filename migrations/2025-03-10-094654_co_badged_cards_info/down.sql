-- This file should undo anything in `up.sql`
DROP INDEX co_badged_cards_card_bin_min_card_bin_max_index;

DROP TABLE co_badged_cards_info;

DROP TYPE IF EXISTS "CardType";

DROP TYPE IF EXISTS "PanOrToken";