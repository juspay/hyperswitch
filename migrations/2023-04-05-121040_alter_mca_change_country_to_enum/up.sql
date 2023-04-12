ALTER TABLE merchant_connector_account
ALTER COLUMN business_country TYPE "CountryCode" USING business_country::"CountryCode";
