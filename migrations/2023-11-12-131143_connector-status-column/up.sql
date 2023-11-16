-- Your SQL goes here
CREATE TYPE "ConnectorStatus" AS ENUM ('active', 'inactive');

ALTER TABLE merchant_connector_account
ADD COLUMN status "ConnectorStatus";

UPDATE merchant_connector_account SET status='active';

ALTER TABLE merchant_connector_account
ALTER COLUMN status SET NOT NULL,
ALTER COLUMN status SET DEFAULT 'inactive';
