-- Add preferred_connector column to customers table for preferred-connector routing (JSON map keyed by payment method type, each holding {profile_id: "connector:mca_id"} entries; customer's last successful redirect connector)
ALTER TABLE customers ADD COLUMN IF NOT EXISTS preferred_connector JSONB;
