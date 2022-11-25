ALTER TABLE payment_attempt
ADD COLUMN browser_info JSONB DEFAULT '{}'::JSONB;
