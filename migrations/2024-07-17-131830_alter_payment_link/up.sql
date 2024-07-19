-- Rename for naming convention
ALTER table payment_link RENAME column link_to_pay to link_open;

-- Add a new column for allowed domains and secure link endpoint
ALTER table payment_link ADD COLUMN link_secure VARCHAR(255);