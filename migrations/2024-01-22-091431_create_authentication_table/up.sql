CREATE TYPE "DecoupledAuthenticationType" AS ENUM (
    'challenge',
    'frictionless'
);
CREATE TYPE "AuthenticationStatus" AS ENUM(
    'started',
    'pending',
    'success',
    'failed'
);
CREATE TYPE "AuthenticationLifecycleStatus" AS ENUM(
    'used',
    'unused',
    'expired'
);
CREATE TABLE IF NOT EXISTS Authentication (
    authentication_id VARCHAR(64) NOT NULL,
    merchant_id VARCHAR(64) NOT NULL,
    authentication_connector VARCHAR(64) NOT NULL,
    authentication_connector_id VARCHAR(64),
    authentication_data JSONB,
    payment_method_id VARCHAR(64) NOT NULL,
    authentication_type "DecoupledAuthenticationType",
    authentication_status "AuthenticationStatus" NOT NULL,
    authentication_lifecycle_status "AuthenticationLifecycleStatus" NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP,
    modified_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP,
    error_message VARCHAR(64),
    error_code VARCHAR(64),
    PRIMARY KEY (authentication_id)
);
