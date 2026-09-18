ALTER TABLE merchant_account ADD COLUMN apple_pay_certificates JSONB;
ALTER TABLE merchant_account ADD COLUMN apple_pay_certificates_encrypted BYTEA;

ALTER TABLE business_profile ADD COLUMN apple_pay_certificates JSONB;
ALTER TABLE business_profile ADD COLUMN apple_pay_certificates_encrypted BYTEA;

ALTER TABLE merchant_connector_account ADD COLUMN apple_pay_certificates JSONB;
ALTER TABLE merchant_connector_account ADD COLUMN apple_pay_certificates_encrypted BYTEA;
