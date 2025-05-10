-- Your SQL goes here
CREATE TYPE "CardType" AS ENUM ('credit', 'debit');
CREATE TYPE "PanOrToken" AS ENUM ('pan', 'token');

CREATE TABLE co_badged_cards_info (
    id VARCHAR(64) PRIMARY KEY,
    card_bin_min BIGINT NOT NULL,
    card_bin_max BIGINT NOT NULL,
    issuing_bank_name TEXT,
    card_network VARCHAR(32) NOT NULL,
    country "CountryAlpha2" NOT NULL,
    card_type "CardType" NOT NULL,
    regulated BOOLEAN NOT NULL,
    regulated_name TEXT,
    prepaid BOOLEAN NOT NULL,
    reloadable BOOLEAN NOT NULL,
    pan_or_token "PanOrToken" NOT NULL,
    card_bin_length SMALLINT NOT NULL,
    card_brand_is_additional BOOLEAN NOT NULL,
    domestic_only BOOLEAN NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP,
    modified_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP,
    last_updated_provider VARCHAR(128)
);

CREATE INDEX co_badged_cards_card_bin_min_card_bin_max_index ON co_badged_cards_info (card_bin_min, card_bin_max);