ALTER TABLE merchant_account
DROP COLUMN IF EXISTS token_locker_id,
DROP COLUMN IF EXISTS locker_name;