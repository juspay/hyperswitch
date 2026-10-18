ALTER TABLE merchant_account DROP COLUMN IF EXISTS apple_pay_certificates;
ALTER TABLE merchant_account DROP COLUMN IF EXISTS apple_pay_certificates_encrypted;

ALTER TABLE business_profile DROP COLUMN IF EXISTS apple_pay_certificates;
ALTER TABLE business_profile DROP COLUMN IF EXISTS apple_pay_certificates_encrypted;

ALTER TABLE merchant_connector_account DROP COLUMN IF EXISTS apple_pay_certificates;
ALTER TABLE merchant_connector_account DROP COLUMN IF EXISTS apple_pay_certificates_encrypted;
