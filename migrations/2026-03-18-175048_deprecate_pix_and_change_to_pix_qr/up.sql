-- Your SQL goes here
UPDATE payment_attempt
SET payment_method_data = jsonb_set(
    payment_method_data #- '{bank_transfer,pix}', 
    '{bank_transfer,pix_qr}', 
    payment_method_data->'bank_transfer'->'pix',
    true
)
WHERE payment_method_type = 'pix' 
  AND payment_method_data->'bank_transfer' ? 'pix';
  
UPDATE dynamic_routing_stats
SET payment_method_type = 'pix_qr'
WHERE payment_method_type = 'pix';

UPDATE payment_methods
SET payment_method_type = 'pix_qr'
WHERE payment_method_type = 'pix';

UPDATE payment_attempt
SET payment_method_type = 'pix_qr'
WHERE payment_method_type = 'pix';

UPDATE payment_attempt
SET straight_through_algorithm = jsonb_set(
    straight_through_algorithm,
    '{pre_routing_results}',
    (straight_through_algorithm -> 'pre_routing_results' || 
     jsonb_build_object('pix_qr', straight_through_algorithm -> 'pre_routing_results' -> 'pix')) 
     - 'pix'
)
WHERE straight_through_algorithm #> '{pre_routing_results}' ? 'pix';

UPDATE merchant_connector_account
SET metadata = ((metadata::jsonb - 'pix') || jsonb_build_object('pix_qr', metadata::jsonb->'pix'))::json
WHERE metadata::text LIKE '%"pix":%';

UPDATE merchant_connector_account
SET payment_methods_enabled = (
    SELECT array_agg(updated_json::json)
    FROM (
        SELECT 
            jsonb_set(
                elem::jsonb, 
                '{payment_method_types}', 
                (
                    SELECT jsonb_agg(
                        CASE 
                            WHEN pm_type->>'payment_method_type' = 'pix' 
                            THEN pm_type || '{"payment_method_type": "pix_qr"}'::jsonb
                            ELSE pm_type 
                        END
                    )
                    FROM jsonb_array_elements(elem::jsonb->'payment_method_types') AS pm_type
                )
            ) AS updated_json
        FROM unnest(payment_methods_enabled) AS elem
    ) s
)
WHERE payment_methods_enabled::text ILIKE '%pix%';