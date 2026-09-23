-- Add preferred_gateways column to payment_methods table for preferred-gateway routing (JSON map keyed by payment method type, each holding {key: profile_id, value: "connector:mca_id"} entries; payment method's last successful redirect connector)
ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS preferred_gateways JSONB;
