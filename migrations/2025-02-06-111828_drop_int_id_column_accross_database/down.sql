-- This file contains queries to re-create the `id` column as a `SERIAL` column for tables that had it removed.
-- It must be ensured that the deployed version of the application includes the `id` column in any of its queries.
-- Re-create the id column as this was used as the primary key with a different type
------------------------ Merchant Account -----------------------
ALTER TABLE merchant_account ADD COLUMN id SERIAL;

------------------------ Merchant Connector Account -----------------------
ALTER TABLE merchant_connector_account ADD COLUMN id SERIAL;

------------------------ Customers -----------------------
ALTER TABLE customers ADD COLUMN id SERIAL;

------------------------ Payment Intent -----------------------
ALTER TABLE payment_intent ADD COLUMN id SERIAL;

------------------------ Payment Attempt -----------------------
ALTER TABLE payment_attempt ADD COLUMN id SERIAL;

------------------------ Payment Methods -----------------------
ALTER TABLE payment_methods ADD COLUMN id SERIAL;

------------------------ Address -----------------------
ALTER TABLE address ADD COLUMN id SERIAL;

------------------------ Dispute -----------------------
ALTER TABLE dispute ADD COLUMN id SERIAL;

------------------------ Mandate -----------------------
ALTER TABLE mandate ADD COLUMN id SERIAL;

------------------------ Refund -----------------------
ALTER TABLE refund ADD COLUMN id SERIAL;

------------------------ BlockList -----------------------
ALTER TABLE blocklist ADD COLUMN id SERIAL;

------------------------ Blocklist Fingerprint -----------------------
ALTER TABLE blocklist_fingerprint DROP CONSTRAINT blocklist_fingerprint_pkey;
ALTER TABLE blocklist_fingerprint ADD COLUMN id SERIAL PRIMARY KEY;

------------------------ Blocklist Lookup -----------------------
ALTER TABLE blocklist_lookup DROP CONSTRAINT blocklist_lookup_pkey;
ALTER TABLE blocklist_lookup ADD COLUMN id SERIAL PRIMARY KEY;

------------------------ Configs -----------------------
ALTER TABLE configs ADD COLUMN id SERIAL;

------------------------ Roles -----------------------
ALTER TABLE roles ADD COLUMN id SERIAL;

------------------------ Users -----------------------
ALTER TABLE users ADD COLUMN id SERIAL;

---------------------- Locker Mockup -----------------------
ALTER TABLE locker_mock_up DROP CONSTRAINT locker_mock_up_pkey;
ALTER TABLE locker_mock_up ADD COLUMN id SERIAL PRIMARY KEY;
