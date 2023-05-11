CREATE type "EntityType" as ENUM (
    -- Adyen
    'Individual',
    'Company',
    'NonProfit',
    'PublicSector',

    -- Wise
    'business',
    'personal'
);
CREATE type "PayoutStatus" AS ENUM (
    'success',
    'failed',
    'cancelled',
    'pending',
    'ineligible',
    'requires_creation',
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
    payout_method_data JSONB,
    amount BIGINT NOT NULL,
    destination_currency "Currency" NOT NULL,
    source_currency "Currency" NOT NULL,
    description VARCHAR (255),
    recurring BOOLEAN NOT NULL,
    auto_fulfill BOOLEAN NOT NULL,
    return_url VARCHAR (255),
    entity_type "EntityType" NOT NULL,
    metadata JSONB DEFAULT '{}'::JSONB,
    created_at timestamp NOT NULL DEFAULT NOW()::timestamp,
    last_modified_at timestamp NOT NULL DEFAULT NOW()::timestamp
);
CREATE TABLE PAYOUTS (
    id SERIAL PRIMARY KEY,
    payout_id VARCHAR (64) NOT NULL,
    customer_id VARCHAR (64) NOT NULL,
    merchant_id VARCHAR (64) NOT NULL,
    address_id VARCHAR (64) NOT NULL,
    connector VARCHAR (64) NOT NULL,
    connector_payout_id VARCHAR (64) NOT NULL,
    payout_method_data JSONB,
    status "PayoutStatus" NOT NULL,
    is_eligible BOOLEAN,
    encoded_data TEXT,
    error_message TEXT,
    error_code VARCHAR (64)
);