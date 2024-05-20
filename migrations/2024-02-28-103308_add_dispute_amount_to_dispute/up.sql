-- Your SQL goes here
-- Add the new column with a default value
ALTER TABLE dispute
ADD COLUMN dispute_amount BIGINT NOT NULL DEFAULT 0;

-- Update existing rows to set the default value based on the integer equivalent of the amount column
UPDATE dispute
SET dispute_amount = CAST(amount AS BIGINT);
