-- This file contains queries to re-create the `id` column as a `VARCHAR(64)` column for tables that had it removed.
-- It must be ensured that the deployed version of the application includes the `id` column in any of its queries.
-- Re-create the id column as this was used as the primary key with a different type
------------------------ Merchant Account -----------------------
ALTER TABLE merchant_account ADD COLUMN id VARCHAR(64);

------------------------ Merchant Connector Account -----------------------
ALTER TABLE merchant_connector_account ADD COLUMN id VARCHAR(64);

------------------------ Customers -----------------------
ALTER TABLE customers ADD COLUMN id VARCHAR(64);

------------------------ Payment Intent -----------------------
ALTER TABLE payment_intent ADD COLUMN id VARCHAR(64);

------------------------ Payment Attempt -----------------------
ALTER TABLE payment_attempt ADD COLUMN id VARCHAR(64);

------------------------ Payment Methods -----------------------
ALTER TABLE payment_methods ADD COLUMN id VARCHAR(64);

------------------------ Address -----------------------
ALTER TABLE address ADD COLUMN id VARCHAR(64);

------------------------ Dispute -----------------------
ALTER TABLE dispute ADD COLUMN id VARCHAR(64);

------------------------ Mandate -----------------------
ALTER TABLE mandate ADD COLUMN id VARCHAR(64);

------------------------ Refund -----------------------
ALTER TABLE refund ADD COLUMN id VARCHAR(64);

------------------------ BlockList -----------------------
ALTER TABLE blocklist ADD COLUMN id VARCHAR(64);

------------------------ Roles -----------------------
ALTER TABLE roles ADD COLUMN id VARCHAR(64);

------------------------ Users -----------------------
ALTER TABLE users ADD COLUMN id VARCHAR(64);
