use common_utils::errors::CustomResult;
use diesel_models::dynamic_routing_stats;
use error_stack::report;
use hyperswitch_domain_models::db::dynamic_routing_stats::DynamicRoutingStatsInterface;
use router_env::{instrument, tracing};

// Import `connection` for RouterStore to get DB connections
use crate::{connection, errors, kv_router_store::KVRouterStore, mock_db::MockDb, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl DynamicRoutingStatsInterface for MockDb {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn insert_dynamic_routing_stat_entry(
        &self,
        _dynamic_routing_stat: dynamic_routing_stats::DynamicRoutingStatsNew,
    ) -> CustomResult<dynamic_routing_stats::DynamicRoutingStats, Self::Error> {
        Err(report!(errors::StorageError::MockDbError).attach_printable("Mock DB error: insert_dynamic_routing_stat_entry not implemented"))
    }

    async fn find_dynamic_routing_stats_optional_by_attempt_id_merchant_id(
        &self,
        _attempt_id: String,
        _merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Option<dynamic_routing_stats::DynamicRoutingStats>, Self::Error> {
        Err(report!(errors::StorageError::MockDbError).attach_printable("Mock DB error: find_dynamic_routing_stats_optional_by_attempt_id_merchant_id not implemented"))
    }

    async fn update_dynamic_routing_stats(
        &self,
        _attempt_id: String,
        _merchant_id: &common_utils::id_type::MerchantId,
        _data: dynamic_routing_stats::DynamicRoutingStatsUpdate,
    ) -> CustomResult<dynamic_routing_stats::DynamicRoutingStats, Self::Error> {
        Err(report!(errors::StorageError::MockDbError).attach_printable("Mock DB error: update_dynamic_routing_stats not implemented"))
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore + Sync> DynamicRoutingStatsInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn insert_dynamic_routing_stat_entry(
        &self,
        dynamic_routing_stat: dynamic_routing_stats::DynamicRoutingStatsNew,
    ) -> CustomResult<dynamic_routing_stats::DynamicRoutingStats, Self::Error> {
        let conn = connection::pg_connection_write(self).await?;
        dynamic_routing_stat
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    async fn find_dynamic_routing_stats_optional_by_attempt_id_merchant_id(
        &self,
        attempt_id: String,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Option<dynamic_routing_stats::DynamicRoutingStats>, Self::Error> {
        let conn = connection::pg_connection_read(self).await?;
        dynamic_routing_stats::DynamicRoutingStats::find_optional_by_attempt_id_merchant_id(
            &conn,
            attempt_id,
            merchant_id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    async fn update_dynamic_routing_stats(
        &self,
        attempt_id: String,
        merchant_id: &common_utils::id_type::MerchantId,
        data: dynamic_routing_stats::DynamicRoutingStatsUpdate,
    ) -> CustomResult<dynamic_routing_stats::DynamicRoutingStats, Self::Error> {
        let conn = connection::pg_connection_write(self).await?;
        dynamic_routing_stats::DynamicRoutingStats::update(&conn, attempt_id, merchant_id, data)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore + Sync> DynamicRoutingStatsInterface for KVRouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn insert_dynamic_routing_stat_entry(
        &self,
        dynamic_routing_stat: dynamic_routing_stats::DynamicRoutingStatsNew,
    ) -> CustomResult<dynamic_routing_stats::DynamicRoutingStats, Self::Error> {
        // KV store should abstract away the direct DB interaction via RouterStore
        self.router_store
            .insert_dynamic_routing_stat_entry(dynamic_routing_stat)
            .await
    }

    async fn find_dynamic_routing_stats_optional_by_attempt_id_merchant_id(
        &self,
        attempt_id: String,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Option<dynamic_routing_stats::DynamicRoutingStats>, Self::Error> {
        // KV store should abstract away the direct DB interaction via RouterStore
        // Potentially add KV caching logic here in the future if needed for this interface
        self.router_store
            .find_dynamic_routing_stats_optional_by_attempt_id_merchant_id(
                attempt_id,
                merchant_id,
            )
            .await
    }

    async fn update_dynamic_routing_stats(
        &self,
        attempt_id: String,
        merchant_id: &common_utils::id_type::MerchantId,
        data: dynamic_routing_stats::DynamicRoutingStatsUpdate,
    ) -> CustomResult<dynamic_routing_stats::DynamicRoutingStats, Self::Error> {
        // KV store should abstract away the direct DB interaction via RouterStore
        self.router_store
            .update_dynamic_routing_stats(attempt_id, merchant_id, data)
            .await
    }
}
