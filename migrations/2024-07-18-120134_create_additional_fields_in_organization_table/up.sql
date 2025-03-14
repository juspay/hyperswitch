-- Your SQL goes here
ALTER TABLE organization
ADD COLUMN organization_details jsonb,
ADD COLUMN metadata jsonb,
ADD created_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP,
ADD modified_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP;