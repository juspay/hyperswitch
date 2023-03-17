CREATE TYPE "DisputeStage" AS ENUM ('pre_dispute', 'dispute', 'pre_arbitration');

CREATE TYPE "DisputeStatus" AS ENUM ('disput_opened', 'dispute_expired', 'dispute_accepted', 'dispute_cancelled', 'dispute_challenged', 'dispute_won', 'dispute_lost');

CREATE TABLE dispute (
    id SERIAL PRIMARY KEY,
    dispute_id VARCHAR(255) NOT NULL,
    amount VARCHAR(255) NOT NULL,
    currency VARCHAR(255) NOT NULL,
    dispute_stage "DisputeStage" NOT NULL,
    dispute_status "DisputeStatus" NOT NULL,
    payment_id VARCHAR(255) NOT NULL,
    merchant_id VARCHAR(255) NOT NULL,
    connector_status VARCHAR(255) NOT NULL,
    connector_dispute_id VARCHAR(255) NOT NULL,
    connector_reason VARCHAR(255),
    connector_reason_code VARCHAR(255),
    challenge_required_by VARCHAR(255),
    dispute_created_at VARCHAR(255),
    updated_at VARCHAR(255),
    created_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP,
    modified_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP
);

ALTER TYPE "EventClass" ADD VALUE 'disputes';

ALTER TYPE "EventObjectType" ADD VALUE 'dispute_details';

ALTER TYPE "EventType" ADD VALUE 'dispute_opened';
ALTER TYPE "EventType" ADD VALUE 'dispute_expired';
ALTER TYPE "EventType" ADD VALUE 'dispute_accepted';
ALTER TYPE "EventType" ADD VALUE 'dispute_cancelled';
ALTER TYPE "EventType" ADD VALUE 'dispute_challenged';
ALTER TYPE "EventType" ADD VALUE 'dispute_won';
ALTER TYPE "EventType" ADD VALUE 'dispute_lost';
