/*
 We could have added the `hash_key` column with a default of the plaintext key
 used for hashing API keys, but we don't do that as it is a hassle to update
 this migration with the plaintext hash key.
 */
TRUNCATE TABLE api_keys;

ALTER TABLE api_keys
ADD COLUMN hash_key VARCHAR(64) NOT NULL;

ALTER TABLE api_keys DROP CONSTRAINT api_keys_hashed_api_key_key;
