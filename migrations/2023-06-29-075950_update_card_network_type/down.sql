-- This file should undo anything in `up.sql`

DROP TYPE "CardNetwork";

ALTER TABLE cards_info
ALTER COLUMN card_network TYPE Text;
