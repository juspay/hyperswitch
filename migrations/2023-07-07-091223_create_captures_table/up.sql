
CREATE TYPE "CaptureStatus" AS ENUM (
    'started',
    'charged',
    'pending',
    'failed'
);
ALTER TYPE "IntentStatus" ADD VALUE If NOT EXISTS 'partially_captured' AFTER 'requires_capture';
CREATE TABLE captures(
    capture_id VARCHAR(64) NOT NULL PRIMARY KEY,
    payment_id VARCHAR(64) NOT NULL,
    merchant_id VARCHAR(64) NOT NULL,
    status "CaptureStatus" NOT NULL,
    amount BIGINT NOT NULL,
    currency "Currency",
    connector VARCHAR(255),
    error_message VARCHAR(255),
    error_code VARCHAR(255),
    error_reason VARCHAR(255),
    tax_amount BIGINT,
    created_at TIMESTAMP NOT NULL,
    modified_at TIMESTAMP NOT NULL,
    authorized_attempt_id VARCHAR(64) NOT NULL,
    connector_transaction_id VARCHAR(128),
    capture_sequence SMALLINT NOT NULL
);

CREATE INDEX authorized_attempt_id_index ON captures (authorized_attempt_id);
CREATE INDEX connector_transaction_id_index ON captures (
    merchant_id,
    payment_id,
    connector_transaction_id
);

ALTER TABLE payment_attempt
ADD COLUMN multiple_capture_count SMALLINT; --number of captures available for this payment attempt in captures table
