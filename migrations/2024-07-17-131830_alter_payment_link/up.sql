-- Add a new column for allowed domains and secure link endpoint
ALTER table payment_link ADD COLUMN link_secure VARCHAR(255);