-- Migration: Add standardised_code and error_category to payment_attempts
-- These columns enable normalized decline analytics using GSM standardised codes

-- Step 1: Add columns to Kafka queue table
ALTER TABLE payment_attempt_queue
    ADD COLUMN IF NOT EXISTS `standardised_code` LowCardinality(Nullable(String)) AFTER `processor_merchant_id`,
    ADD COLUMN IF NOT EXISTS `error_category` LowCardinality(Nullable(String)) AFTER `standardised_code`;

-- Step 2: Add columns to storage table
ALTER TABLE payment_attempts
    ADD COLUMN IF NOT EXISTS `standardised_code` LowCardinality(Nullable(String)) AFTER `processor_merchant_id`,
    ADD COLUMN IF NOT EXISTS `error_category` LowCardinality(Nullable(String)) AFTER `standardised_code`;

-- Step 3: Add bloom filter indexes
ALTER TABLE payment_attempts
    ADD INDEX IF NOT EXISTS standardisedCodeIndex standardised_code TYPE bloom_filter GRANULARITY 1,
    ADD INDEX IF NOT EXISTS errorCategoryIndex error_category TYPE bloom_filter GRANULARITY 1;

-- Step 4: Drop and recreate materialized view to include new columns
-- WARNING: Brief data gap during MV recreation. Schedule during low-traffic window.
-- Kafka consumer will backfill any missed events.
DROP VIEW IF EXISTS payment_attempt_mv;

-- The recreated MV is defined in payment_attempts.sql
-- Run the CREATE MATERIALIZED VIEW statement from the updated payment_attempts.sql
