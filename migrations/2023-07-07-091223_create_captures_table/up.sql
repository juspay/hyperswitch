
CREATE TYPE "CaptureStatus" AS ENUM (
    'started',
    'charged',
    'pending',
    'failure'
);
CREATE TABLE captures(
    capture_id VARCHAR(255) NOT NULL PRIMARY KEY,
    payment_id VARCHAR(255) NOT NULL,
    merchant_id VARCHAR(255) NOT NULL,
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
    authorized_attempt_id VARCHAR(255) NOT NULL,
    capture_sequence SMALLINT NOT NULL
);