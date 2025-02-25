
use common_utils::errors::CustomResult;
use diesel_models::routing_algorithm as storage;
use error_stack::report;
use router_env::{instrument, tracing};
use sample::routing_algorithm::RoutingAlgorithmInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> RoutingAlgorithmInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn insert_routing_algorithm(
        &self,
        routing_algorithm: storage::RoutingAlgorithm,
    ) -> CustomResult<storage::RoutingAlgorithm, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        routing_algorithm
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_routing_algorithm_by_profile_id_algorithm_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        algorithm_id: &common_utils::id_type::RoutingId,
    ) -> CustomResult<storage::RoutingAlgorithm, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::RoutingAlgorithm::find_by_algorithm_id_profile_id(
            &conn,
            algorithm_id,
            profile_id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_routing_algorithm_by_algorithm_id_merchant_id(
        &self,
        algorithm_id: &common_utils::id_type::RoutingId,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<storage::RoutingAlgorithm, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::RoutingAlgorithm::find_by_algorithm_id_merchant_id(
            &conn,
            algorithm_id,
            merchant_id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_routing_algorithm_metadata_by_algorithm_id_profile_id(
        &self,
        algorithm_id: &common_utils::id_type::RoutingId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<storage::RoutingProfileMetadata, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::RoutingAlgorithm::find_metadata_by_algorithm_id_profile_id(
            &conn,
            algorithm_id,
            profile_id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_routing_algorithm_metadata_by_profile_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::RoutingProfileMetadata>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::RoutingAlgorithm::list_metadata_by_profile_id(
            &conn, profile_id, limit, offset,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_routing_algorithm_metadata_by_merchant_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::RoutingProfileMetadata>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::RoutingAlgorithm::list_metadata_by_merchant_id(
            &conn,
            merchant_id,
            limit,
            offset,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    async fn list_routing_algorithm_metadata_by_merchant_id_transaction_type(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        transaction_type: &common_enums::TransactionType,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::RoutingProfileMetadata>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::RoutingAlgorithm::list_metadata_by_merchant_id_transaction_type(
            &conn,
            merchant_id,
            transaction_type,
            limit,
            offset,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }
}