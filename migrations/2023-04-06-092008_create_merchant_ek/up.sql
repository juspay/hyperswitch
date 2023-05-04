CREATE TABLE merchant_key_store(
    MERCHANT_ID VARCHAR(255) NOT NULL PRIMARY KEY,
    KEY BYTEA NOT NULL,
    CREATED_AT TIMESTAMP NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX merchant_key_store_unique_index ON merchantkeystore(merchant_id);
