ALTER TABLE business_profile
ADD COLUMN IF NOT EXISTS force_3ds_challenge boolean DEFAULT false;