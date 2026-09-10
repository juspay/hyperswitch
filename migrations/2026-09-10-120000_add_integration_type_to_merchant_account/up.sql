-- Which integration the merchant builds its checkout with (`client`, `server` or
-- `client_and_server`). Gates the `X-Integration-Type` header on payment requests.
-- NULL reads as `client_and_server`, which accepts either header value.
ALTER TABLE merchant_account
ADD COLUMN IF NOT EXISTS integration_type VARCHAR(32);
