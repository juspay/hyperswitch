-- Update sepa to sepa_bank_transfer in payment methods and routing algorithms
DO $$
BEGIN
    -- Update merchant connector accounts (only if payout_processor enum exists)
    IF EXISTS (
        SELECT 1 FROM pg_enum e
        JOIN pg_type t ON e.enumtypid = t.oid
        WHERE t.typname = 'ConnectorType' AND e.enumlabel = 'payout_processor'
    ) THEN
        UPDATE merchant_connector_account
        SET payment_methods_enabled = sepa_updated.updated_methods
        FROM (
            SELECT
                merchant_connector_id,
                array_agg(
                    CASE
                        WHEN method::text LIKE '%"sepa"%' THEN 
                            replace(method::text, '"sepa"', '"sepa_bank_transfer"')::json
                        ELSE method
                    END
                ) AS updated_methods
            FROM merchant_connector_account, unnest(payment_methods_enabled) AS method
            WHERE payment_methods_enabled IS NOT NULL
              AND connector_type::text = 'payout_processor'
            GROUP BY merchant_connector_id
        ) AS sepa_updated
        WHERE merchant_connector_account.merchant_connector_id = sepa_updated.merchant_connector_id
          AND connector_type::text = 'payout_processor';
    END IF;
    
    -- Update routing algorithms
    UPDATE routing_algorithm
    SET algorithm_data = replace(algorithm_data::text, '"sepa"', '"sepa_bank_transfer"')::jsonb
    WHERE algorithm_for = 'payout' AND algorithm_data::text LIKE '%"sepa"%';
END $$;
