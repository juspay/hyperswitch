ALTER TABLE merchant_account
    DROP COLUMN google_pay_decryption_config_encrypted,
    DROP COLUMN google_pay_decryption_config;

ALTER TABLE business_profile
    DROP COLUMN google_pay_decryption_config_encrypted,
    DROP COLUMN google_pay_decryption_config;
