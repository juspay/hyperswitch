CREATE type "PayoutStatus" AS ENUM (
    'succeeded',
    'failed',
    'cancelled',
    'processing',
    'requires_fulfillment'
);
CREATE type "PayoutType" AS ENUM ('card', 'bank');
CREATE TABLE PAYOUT_CREATE (
    id serial PRIMARY KEY,
    payout_id VARCHAR (64) NOT NULL,
    merchant_id VARCHAR (64) NOT NULL,
    customer_id VARCHAR (64) NOT NULL,
    address_id VARCHAR (64) NOT NULL,
    payout_type "PayoutType" NOT NULL,
    amount BIGINT NOT NULL,
    destination_currency "Currency" NOT NULL,
    source_currency "Currency" NOT NULL,
    description VARCHAR (255),
    created_at timestamp NOT NULL DEFAULT NOW()::timestamp,
    modified_at timestamp NOT NULL DEFAULT NOW()::timestamp,
    status "PayoutStatus" NOT NULL,
    metadata jsonb DEFAULT '{}'::jsonb,
    recurring boolean NOT NULL DEFAULT false,
    connector VARCHAR(64) NOT NULL,
    error_message TEXT,
    error_code VARCHAR(255) 
);
CREATE TABLE PAYOUTS (
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