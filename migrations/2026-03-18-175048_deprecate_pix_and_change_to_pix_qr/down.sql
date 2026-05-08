-- This file should undo anything in `up.sql`

-- 1. Revert payment_method_data in payment_attempt
UPDATE payment_attempt
SET payment_method_data = jsonb_set(
    payment_method_data #- '{bank_transfer,pix_emv}', 
    '{bank_transfer,pix}', 
    payment_method_data->'bank_transfer'->'pix_emv',
    true
)
WHERE payment_method_type = 'pix_emv' 
  AND payment_method_data->'bank_transfer' ? 'pix_emv' 
  AND connector = 'santander'; 

-- 2. Revert payment_method_type in payment_attempt
UPDATE payment_attempt
SET payment_method_type = 'pix'
WHERE payment_method_type = 'pix_emv' 
  AND connector = 'santander';

-- 3. Revert straight_through_algorithm in payment_attempt
UPDATE payment_attempt
SET straight_through_algorithm = jsonb_set(
    straight_through_algorithm,
    '{pre_routing_results}',
    (straight_through_algorithm -> 'pre_routing_results' || 
     jsonb_build_object('pix', straight_through_algorithm -> 'pre_routing_results' -> 'pix_emv')) 
     - 'pix_emv'
)
WHERE straight_through_algorithm #> '{pre_routing_results}' ? 'pix_emv' 
  AND connector = 'santander'; 

-- 4. Revert metadata in merchant_connector_account
UPDATE merchant_connector_account
SET metadata = (metadata - 'pix_emv') || jsonb_build_object('pix', metadata->'pix_emv')
WHERE connector_name = 'santander' 
  AND metadata ? 'pix_emv';

-- 5. Revert payment_methods_enabled in merchant_connector_account
UPDATE merchant_connector_account
SET payment_methods_enabled = REPLACE(
    payment_methods_enabled::TEXT, 
    '"payment_method_type":"pix_emv"', 
    '"payment_method_type":"pix"'
)::JSONB
WHERE connector_name = 'santander'
  AND payment_methods_enabled::TEXT ILIKE '%"payment_method_type"%pix_emv%';