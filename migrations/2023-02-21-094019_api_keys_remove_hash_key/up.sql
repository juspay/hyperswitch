ALTER TABLE api_keys DROP COLUMN hash_key;

/*
 Once we've dropped the `hash_key` column, we cannot use the existing API keys
 from the `api_keys` table anymore, as the `hash_key` is a random string that
 we no longer have.
 */
TRUNCATE TABLE api_keys;

ALTER TABLE api_keys
ADD CONSTRAINT api_keys_hashed_api_key_key UNIQUE (hashed_api_key);
