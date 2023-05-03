CREATE TYPE "PayoutStatus" AS ENUM ('created', 'pending', 'success', 'failed');
CREATE TYPE "PayoutType" AS ENUM ('card', 'bank');

CREATE TABLE payout_create (
    id SERIAL PRIMARY KEY,
    payout_id VARCHAR(255) NOT NULL,
    customer_id VARCHAR(255) NOT NULL,
    merchant_id VARCHAR(255) NOT NULL,
    status "PayoutStatus" NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP,
    encoded_data JSONB,
    connector VARCHAR(255) NOT NULL,
    error_message TEXT,
    error_code VARCHAR(255) 
);

CREATE TABLE payouts (
    id SERIAL PRIMARY KEY,
    payout_id VARCHAR(255) NOT NULL,
    customer_id VARCHAR(255) NOT NULL,
    merchant_id VARCHAR(255) NOT NULL,
    address_id VARCHAR(255) NOT NULL,
    payout_type "PayoutType" NOT NULL,
    connector_payout_id VARCHAR(255) NOT NULL,
    connector VARCHAR(255) NOT NULL,
    payout_data JSONB,
    amount bigint NOT NULL,
    destination_currency "Currency" NOT NULL,
    source_currency "Currency" NOT NULL,
    recurring BOOLEAN
);