-- Revert Santander Pix QR payment method type back to Pix EMV.
UPDATE payment_attempt
SET payment_method_data = jsonb_set(
    payment_method_data #- '{bank_transfer,pix_qr}',
    '{bank_transfer,pix_emv}',
    payment_method_data->'bank_transfer'->'pix_qr',
    true
)
WHERE payment_method_type = 'pix_qr'
  AND payment_method_data->'bank_transfer' ? 'pix_qr'
  AND connector = 'santander';

UPDATE payment_attempt
SET payment_method_type = 'pix_emv'
WHERE payment_method_type = 'pix_qr'
  AND connector = 'santander';

UPDATE payment_attempt
SET straight_through_algorithm = jsonb_set(
    straight_through_algorithm,
    '{pre_routing_results}',
    (straight_through_algorithm -> 'pre_routing_results' ||
     jsonb_build_object('pix_emv', straight_through_algorithm -> 'pre_routing_results' -> 'pix_qr'))
     - 'pix_qr'
)
WHERE straight_through_algorithm #> '{pre_routing_results}' ? 'pix_qr'
  AND connector = 'santander';

UPDATE merchant_connector_account
SET metadata = (metadata - 'pix_qr') || jsonb_build_object('pix_emv', metadata->'pix_qr')
WHERE connector_name = 'santander'
  AND metadata ? 'pix_qr';

WITH updated_data AS (
    SELECT
        id,
        array_agg(
            REPLACE(elem::text, '"payment_method_type":"pix_qr"', '"payment_method_type":"pix_emv"')::json
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
