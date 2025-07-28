-- Migration to update sepa to sepa_bank_transfer in merchant_connector_account for payout processors
UPDATE merchant_connector_account mca
SET
    payment_methods_enabled = updated.new_array
FROM (
        SELECT
            merchant_connector_id, array_agg(
                CASE
                    WHEN elem IS NOT NULL
                    AND elem::text LIKE '%"sepa"%' THEN replace(
                        elem::text, '"sepa"', '"sepa_bank_transfer"'
                    )::json
                    ELSE elem
                END
            ) AS new_array
        FROM
            merchant_connector_account, unnest(payment_methods_enabled) AS elem
        WHERE
            payment_methods_enabled IS NOT NULL
            AND connector_type = 'payout_processor'
            AND elem IS NOT NULL
            AND elem::text LIKE '%"sepa"%'
        GROUP BY
            merchant_connector_id
    ) AS updated
WHERE
    mca.merchant_connector_id = updated.merchant_connector_id
    AND mca.connector_type = 'payout_processor';

-- Migration to update sepa to sepa_bank_transfer in business_profile for payout routing algorithm
UPDATE routing_algorithm ra
SET
    algorithm_data = replace(
        algorithm_data::text,
        '"sepa"',
        '"sepa_bank_transfer"'
    )::jsonb
WHERE
    algorithm_id IN (
        SELECT DISTINCT
            payout_routing_algorithm ->> 'algorithm_id'
        FROM business_profile
        WHERE
            payout_routing_algorithm IS NOT NULL
            AND payout_routing_algorithm ->> 'algorithm_id' IS NOT NULL
    )
    AND algorithm_data::text LIKE '%"sepa"%';