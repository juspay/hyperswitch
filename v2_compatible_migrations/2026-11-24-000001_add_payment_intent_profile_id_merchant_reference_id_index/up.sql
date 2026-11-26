-- Revenue recovery finds an invoice's payment intent by profile_id and merchant_reference_id
-- for every recovery request; without this index each lookup scans the whole table.
-- A merchant reference id identifies one payment intent within a profile, so the index is unique.
-- Payment intents without a merchant reference id are left out and are not affected.
CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS payment_intent_profile_id_merchant_reference_id_unique_index
ON payment_intent (profile_id, merchant_reference_id)
WHERE merchant_reference_id IS NOT NULL;
