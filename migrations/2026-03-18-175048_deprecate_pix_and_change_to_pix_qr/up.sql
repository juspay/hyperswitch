-- Your SQL goes here
UPDATE payment_attempt
SET payment_method_data = jsonb_set(
    payment_method_data #- '{bank_transfer,pix}', 
    '{bank_transfer,pix_emv}', 
    payment_method_data->'bank_transfer'->'pix',
    true
)
WHERE payment_method_type = 'pix' 
  AND payment_method_data->'bank_transfer' ? 'pix' AND connector = 'santander'; 

UPDATE payment_attempt
SET payment_method_type = 'pix_emv'
WHERE payment_method_type = 'pix' AND connector = 'santander';

UPDATE payment_attempt
SET straight_through_algorithm = jsonb_set(
    straight_through_algorithm,
    '{pre_routing_results}',
    (straight_through_algorithm -> 'pre_routing_results' || 
     jsonb_build_object('pix_emv', straight_through_algorithm -> 'pre_routing_results' -> 'pix')) 
     - 'pix'
)
WHERE straight_through_algorithm #> '{pre_routing_results}' ? 'pix' 
  AND connector = 'santander';

UPDATE merchant_connector_account
SET metadata = (metadata - 'pix') || jsonb_build_object('pix_emv', metadata->'pix')
WHERE connector_name = 'santander' 
  AND metadata ? 'pix';

WITH updated_data AS (
    SELECT 
        id,
        array_agg(
            REPLACE(elem::text, '"payment_method_type":"pix"', '"payment_method_type":"pix_emv"')::json
        ) AS new_array
    FROM 
        merchant_connector_account,
        unnest(payment_methods_enabled) AS elem
    WHERE 
        connector_name = 'santander'
    GROUP BY 
        id
)
UPDATE merchant_connector_account m
SET payment_methods_enabled = u.new_array
FROM updated_data u
WHERE m.id = u.id;