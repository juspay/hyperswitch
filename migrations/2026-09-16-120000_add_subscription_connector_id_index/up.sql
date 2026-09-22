CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS subscription_merchant_connector_subscription_id_index
    ON subscription (merchant_id, merchant_connector_id, connector_subscription_id)
    WHERE connector_subscription_id IS NOT NULL;
