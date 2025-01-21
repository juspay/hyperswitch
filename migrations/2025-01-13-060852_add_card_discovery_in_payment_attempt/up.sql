-- Your SQL goes here
CREATE TYPE "CardDiscovery" AS ENUM ('manual', 'saved_card', 'click_to_pay');

ALTER TABLE payment_attempt ADD COLUMN IF NOT EXISTS card_discovery "CardDiscovery";
