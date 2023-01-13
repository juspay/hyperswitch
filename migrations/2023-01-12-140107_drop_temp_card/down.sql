CREATE TABLE temp_card (
    id SERIAL PRIMARY KEY,
    date_created TIMESTAMP NOT NULL,
    txn_id VARCHAR(255),
    card_info JSON
);

CREATE INDEX temp_card_txn_id_index ON temp_card (txn_id);
