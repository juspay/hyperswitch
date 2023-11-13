-- Your SQL goes here
CREATE TYPE "ConnectorStatus" AS ENUM ('active', 'inactive');

ALTER TABLE merchant_connector_account
ADD COLUMN status "ConnectorStatus";
