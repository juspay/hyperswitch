CREATE type "PayoutStatus" AS ENUM (
    'success',
    'failed',
    'cancelled',
    'pending',
    'ineligible',
    'requires_creation',
    'requires_payout_method_data',
    'requires_fulfillment'
);

CREATE type "PayoutType" AS ENUM ('card', 'bank');

CREATE TABLE
    PAYOUT_ATTEMPT (
        payout_attempt_id VARCHAR (64) NOT NULL PRIMARY KEY,
        payout_id VARCHAR (64) NOT NULL,
        customer_id VARCHAR (64) NOT NULL,
        merchant_id VARCHAR (64) NOT NULL,
        address_id VARCHAR (64) NOT NULL,
        connector VARCHAR (64) NOT NULL,
        connector_payout_id VARCHAR (128) NOT NULL,
        payout_token VARCHAR (64),
        status "PayoutStatus" NOT NULL,
        is_eligible BOOLEAN,
        error_message TEXT,
        error_code VARCHAR (64),
        business_country "CountryAlpha2",
        business_label VARCHAR(64),
        created_at timestamp NOT NULL DEFAULT NOW():: timestamp,
        last_modified_at timestamp NOT NULL DEFAULT NOW():: timestamp
    );

CREATE TABLE
    PAYOUTS (
        payout_id VARCHAR (64) NOT NULL PRIMARY KEY,
        merchant_id VARCHAR (64) NOT NULL,
        customer_id VARCHAR (64) NOT NULL,
        address_id VARCHAR (64) NOT NULL,
        payout_type "PayoutType" NOT NULL,
        payout_method_id VARCHAR (64),
        amount BIGINT NOT NULL,
        destination_currency "Currency" NOT NULL,
        source_currency "Currency" NOT NULL,
        description VARCHAR (255),
        recurring BOOLEAN NOT NULL,
        auto_fulfill BOOLEAN NOT NULL,
        return_url VARCHAR (255),
        entity_type VARCHAR (64) NOT NULL,
        metadata JSONB DEFAULT '{}':: JSONB,
        created_at timestamp NOT NULL DEFAULT NOW():: timestamp,
        last_modified_at timestamp NOT NULL DEFAULT NOW():: timestamp
    );

CREATE UNIQUE INDEX payout_attempt_index ON PAYOUT_ATTEMPT (
    merchant_id,
    payout_attempt_id,
    payout_id
);

CREATE UNIQUE INDEX payouts_index ON PAYOUTS (merchant_id, payout_id);

-- Alterations

ALTER TABLE merchant_account
ADD
    COLUMN payout_routing_algorithm JSONB;

ALTER TABLE locker_mock_up ADD COLUMN encrypted_card_data TEXT;

ALTER TYPE "ConnectorType" ADD VALUE 'payout_processor';