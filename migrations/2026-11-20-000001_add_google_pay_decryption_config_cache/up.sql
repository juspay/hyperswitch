ALTER TABLE business_profile
    ADD COLUMN google_pay_decryption_config JSONB,
    ADD COLUMN google_pay_decryption_config_encrypted BYTEA;

ALTER TABLE merchant_account
    ADD COLUMN google_pay_decryption_config JSONB,
    ADD COLUMN google_pay_decryption_config_encrypted BYTEA;
