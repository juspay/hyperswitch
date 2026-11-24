-- Revenue recovery finds an invoice's payment intent by profile_id and merchant_reference_id
-- for every recovery request; without this index each lookup scans the whole table.
CREATE INDEX CONCURRENTLY IF NOT EXISTS payment_intent_profile_id_merchant_reference_id_index
ON payment_intent (profile_id, merchant_reference_id)
WHERE merchant_reference_id IS NOT NULL;
