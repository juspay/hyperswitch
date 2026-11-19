use error_stack::report;
use router_env::instrument;
use storage_impl::MockDb;

use super::Store;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    db::kafka_store::KafkaStore,
    types::storage,
};

#[router_derive::track_async_interface]
#[async_trait::async_trait]
pub trait DynamicRoutingStatsInterface {
    async fn insert_dynamic_routing_stat_entry(
        &self,
        dynamic_routing_stat_new: storage::DynamicRoutingStatsNew,
    ) -> CustomResult<storage::DynamicRoutingStats, errors::StorageError>;

    async fn find_dynamic_routing_stats_optional_by_attempt_id_merchant_id(
        &self,
        attempt_id: String,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Option<storage::DynamicRoutingStats>, errors::StorageError>;

    async fn update_dynamic_routing_stats(
        &self,
        attempt_id: String,
        merchant_id: &common_utils::id_type::MerchantId,
        data: storage::DynamicRoutingStatsUpdate,
    ) -> CustomResult<storage::DynamicRoutingStats, errors::StorageError>;
}

#[async_trait::async_trait]
impl DynamicRoutingStatsInterface for Store {
    #[instrument(skip_all)]
    async fn insert_dynamic_routing_stat_entry(
        &self,
        dynamic_routing_stat: storage::DynamicRoutingStatsNew,
    ) -> CustomResult<storage::DynamicRoutingStats, errors::StorageError> {
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
    ) -> CustomResult<Option<storage::DynamicRoutingStats>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::DynamicRoutingStats::find_optional_by_attempt_id_merchant_id(
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
        data: storage::DynamicRoutingStatsUpdate,
    ) -> CustomResult<storage::DynamicRoutingStats, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::DynamicRoutingStats::update(&conn, attempt_id, merchant_id, data)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }
}

#[async_trait::async_trait]
impl DynamicRoutingStatsInterface for MockDb {
    #[instrument(skip_all)]
    async fn insert_dynamic_routing_stat_entry(
        &self,
        _dynamic_routing_stat: storage::DynamicRoutingStatsNew,
    ) -> CustomResult<storage::DynamicRoutingStats, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_dynamic_routing_stats_optional_by_attempt_id_merchant_id(
        &self,
        _attempt_id: String,
        _merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Option<storage::DynamicRoutingStats>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_dynamic_routing_stats(
        &self,
        _attempt_id: String,
        _merchant_id: &common_utils::id_type::MerchantId,
        _data: storage::DynamicRoutingStatsUpdate,
    ) -> CustomResult<storage::DynamicRoutingStats, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}

#[async_trait::async_trait]
impl DynamicRoutingStatsInterface for KafkaStore {
    #[instrument(skip_all)]
    async fn insert_dynamic_routing_stat_entry(
        &self,
        dynamic_routing_stat: storage::DynamicRoutingStatsNew,
    ) -> CustomResult<storage::DynamicRoutingStats, errors::StorageError> {
        DynamicRoutingStatsInterface::insert_dynamic_routing_stat_entry(
            &self.diesel_store,
            dynamic_routing_stat,
        )
        .await
    }

    async fn find_dynamic_routing_stats_optional_by_attempt_id_merchant_id(
        &self,
        attempt_id: String,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Option<storage::DynamicRoutingStats>, errors::StorageError> {
        DynamicRoutingStatsInterface::find_dynamic_routing_stats_optional_by_attempt_id_merchant_id(
            &self.diesel_store,
            attempt_id,
            merchant_id,
        )
        .await
    }

    async fn update_dynamic_routing_stats(
        &self,
        attempt_id: String,
        merchant_id: &common_utils::id_type::MerchantId,
        data: storage::DynamicRoutingStatsUpdate,
    ) -> CustomResult<storage::DynamicRoutingStats, errors::StorageError> {
        DynamicRoutingStatsInterface::update_dynamic_routing_stats(
            &self.diesel_store,
            attempt_id,
            merchant_id,
            data,
        )
        .await
    }
}
