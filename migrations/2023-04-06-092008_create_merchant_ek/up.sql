CREATE TABLE MERCHANTKEYSTORE(
    ID SERIAL PRIMARY KEY,
    MERCHANT_ID VARCHAR(255) NOT NULL,
    KEY BYTEA NOT NULL
);
