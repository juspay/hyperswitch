-- Your SQL goes here
CREATE TABLE cards_info (
    card_iin VARCHAR(16) PRIMARY KEY,
    card_issuer TEXT,
    card_network TEXT,
    card_type TEXT,
    card_subtype TEXT,
    card_issuing_country TEXT,
    bank_code_id VARCHAR(32),
    bank_code VARCHAR(32),
    country_code VARCHAR(32),
    date_created TIMESTAMP NOT NULL,
    last_updated TIMESTAMP,
    last_updated_provider TEXT
)
