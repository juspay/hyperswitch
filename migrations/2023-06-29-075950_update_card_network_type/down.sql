-- This file should undo anything in `up.sql`

drop TYPE "CardNetwork";

ALTER TABLE cards_info
ALTER COLUMN card_network TYPE Text;
