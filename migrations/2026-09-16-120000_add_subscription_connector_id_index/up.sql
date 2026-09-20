DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM subscription
        WHERE connector_subscription_id IS NOT NULL
        GROUP BY merchant_id, merchant_connector_id, connector_subscription_id
        HAVING COUNT(*) > 1
    ) THEN
        RAISE EXCEPTION 'duplicate connector subscription bindings must be resolved before migration';
    END IF;
END $$;

CREATE UNIQUE INDEX CONCURRENTLY subscription_merchant_connector_subscription_id_index
    ON subscription (merchant_id, merchant_connector_id, connector_subscription_id)
    WHERE connector_subscription_id IS NOT NULL;
