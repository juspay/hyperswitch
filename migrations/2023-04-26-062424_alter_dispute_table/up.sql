ALTER TABLE dispute
ALTER COLUMN challenge_required_by TYPE TIMESTAMP USING dispute_created_at::TIMESTAMP,
ALTER COLUMN dispute_created_at TYPE TIMESTAMP USING dispute_created_at::TIMESTAMP,
ALTER COLUMN updated_at TYPE TIMESTAMP USING dispute_created_at::TIMESTAMP