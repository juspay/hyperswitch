-- Your SQL goes here

ALTER TABLE routing_algorithm ADD COLUMN IF NOT EXISTS algorithm_type VARCHAR(64) NOT NULL;
