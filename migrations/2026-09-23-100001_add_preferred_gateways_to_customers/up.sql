-- Add preferred_gateways column to customers table for preferred-gateway routing (JSON map keyed by payment method type, each holding {key: profile_id, value: "connector:mca_id"} entries; customer's last successful redirect connector)
ALTER TABLE customers ADD COLUMN IF NOT EXISTS preferred_gateways JSONB;
