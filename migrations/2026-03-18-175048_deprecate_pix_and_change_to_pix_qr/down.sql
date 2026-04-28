-- This file should undo anything in `up.sql`

-- 1. Revert payment_method_data structure in payment_attempt
UPDATE payment_attempt
SET payment_method_data = jsonb_set(
    payment_method_data #- '{bank_transfer,pix_qr}', 
    '{bank_transfer,pix}', 
    payment_method_data->'bank_transfer'->'pix_qr',
    true
)
WHERE payment_method_type = 'pix_qr' 
  AND payment_method_data->'bank_transfer' ? 'pix_qr';

-- 2. Revert payment_method_type in dynamic_routing_stats
UPDATE dynamic_routing_stats
SET payment_method_type = 'pix'
WHERE payment_method_type = 'pix_qr';

-- 3. Revert payment_method_type in payment_methods
UPDATE payment_methods
SET payment_method_type = 'pix'
WHERE payment_method_type = 'pix_qr';

-- 4. Revert payment_method_type in payment_attempt
UPDATE payment_attempt
SET payment_method_type = 'pix'
WHERE payment_method_type = 'pix_qr';

-- 5. Revert pre_routing_results keys in straight_through_algorithm
UPDATE payment_attempt
SET straight_through_algorithm = jsonb_set(
    straight_through_algorithm,
    '{pre_routing_results}',
    (straight_through_algorithm -> 'pre_routing_results' || 
     jsonb_build_object('pix', straight_through_algorithm -> 'pre_routing_results' -> 'pix_qr')) 
     - 'pix_qr'
)
WHERE straight_through_algorithm #> '{pre_routing_results}' ? 'pix_qr';

-- 6. Revert metadata keys in merchant_connector_account
UPDATE merchant_connector_account
SET metadata = ((metadata::jsonb - 'pix_qr') || jsonb_build_object('pix', metadata::jsonb->'pix_qr'))::json
WHERE metadata::text LIKE '%"pix_qr":%';

-- 7. Revert payment_methods_enabled array in merchant_connector_account
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
                            WHEN pm_type->>'payment_method_type' = 'pix_qr' 
                            THEN pm_type || '{"payment_method_type": "pix"}'::jsonb
                            ELSE pm_type 
                        END
                    )
                    FROM jsonb_array_elements(elem::jsonb->'payment_method_types') AS pm_type
                )
            ) AS updated_json
        FROM unnest(payment_methods_enabled) AS elem
    ) s
)
WHERE payment_methods_enabled::text ILIKE '%pix_qr%';