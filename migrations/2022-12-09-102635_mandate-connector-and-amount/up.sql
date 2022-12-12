-- Your SQL goes here
ALTER TABLE mandate
RENAME COLUMN single_use_amount TO mandate_amount;
ALTER TABLE mandate
RENAME COLUMN single_use_currency TO mandate_currency;
ALTER TABLE mandate
ADD IF NOT EXISTS amount_captured INTEGER DEFAULT NULL,
ADD IF NOT EXISTS connector VARCHAR(255) NOT NULL DEFAULT 'dummy',
ADD IF NOT EXISTS connector_mandate_id VARCHAR(255) DEFAULT NULL;