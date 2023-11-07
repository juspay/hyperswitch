-- Your SQL goes here-- Your SQL goes here
CREATE TYPE "FraudCheckType" AS ENUM (
    'pre_frm',
    'post_frm'
);

CREATE TYPE "FraudCheckStatus" AS ENUM (
    'fraud',
    'manual_review',
    'pending',
    'legit',
    'transaction_failure'
);

CREATE TABLE fraud_check (
    frm_id VARCHAR(64) NOT NULL UNIQUE,
    payment_id VARCHAR(64) NOT NULL,
    merchant_id VARCHAR(64) NOT NULL,
    attempt_id VARCHAR(64) NOT NULL UNIQUE,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    frm_name VARCHAR(255) NOT NULL,
    frm_transaction_id VARCHAR(255) UNIQUE,
    frm_transaction_type "FraudCheckType" NOT NULL,
    frm_status "FraudCheckStatus" NOT NULL,
    frm_score INTEGER,
    frm_reason JSONB,
    frm_error VARCHAR(255),
    payment_details JSONB,
    metadata JSONB,
    modified_at TIMESTAMP NOT NULL DEFAULT now(),

    PRIMARY KEY (frm_id, attempt_id, payment_id, merchant_id)
);

CREATE UNIQUE INDEX frm_id_index ON fraud_check (frm_id, attempt_id, payment_id, merchant_id);
