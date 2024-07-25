-- Add a new column for allowed domains and secure link endpoint
ALTER table payment_link ADD COLUMN IF NOT EXISTS secure_link VARCHAR(255);