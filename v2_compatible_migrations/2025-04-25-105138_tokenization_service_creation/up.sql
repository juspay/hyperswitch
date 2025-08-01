-- Your SQL goes here
CREATE TYPE "TokenizationFlag" AS ENUM ('enabled', 'disabled');

CREATE TABLE IF NOT EXISTS tokenization (
    id VARCHAR(64) PRIMARY KEY,  -- GlobalTokenId
    merchant_id VARCHAR(255) NOT NULL,
    customer_id VARCHAR(64) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    locker_id VARCHAR(255) NOT NULL,
    flag "TokenizationFlag" NOT NULL DEFAULT 'enabled',
    version "ApiVersion" NOT NULL DEFAULT 'v2'  -- ApiVersion enum
);