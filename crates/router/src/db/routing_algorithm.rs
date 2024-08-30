use diesel_models::routing_algorithm as routing_storage;
use error_stack::report;
use router_env::{instrument, tracing};
use storage_impl::mock_db::MockDb;

use crate::{
    connection,
    core::errors::{self, CustomResult},
    services::Store,
};

type StorageResult<T> = CustomResult<T, errors::StorageError>;

#[async_trait::async_trait]
pub trait RoutingAlgorithmInterface {
    async fn insert_routing_algorithm(
        &self,
        routing_algorithm: routing_storage::RoutingAlgorithm,
    ) -> StorageResult<routing_storage::RoutingAlgorithm>;

    async fn find_routing_algorithm_by_profile_id_algorithm_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        algorithm_id: &common_utils::id_type::RoutingId,
    ) -> StorageResult<routing_storage::RoutingAlgorithm>;

    async fn find_routing_algorithm_by_algorithm_id_merchant_id(
        &self,
        algorithm_id: &common_utils::id_type::RoutingId,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> StorageResult<routing_storage::RoutingAlgorithm>;

    async fn find_routing_algorithm_metadata_by_algorithm_id_profile_id(
        &self,
        algorithm_id: &common_utils::id_type::RoutingId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> StorageResult<routing_storage::RoutingProfileMetadata>;

    async fn list_routing_algorithm_metadata_by_profile_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<routing_storage::RoutingProfileMetadata>>;

    async fn list_routing_algorithm_metadata_by_merchant_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<routing_storage::RoutingProfileMetadata>>;

    async fn list_routing_algorithm_metadata_by_merchant_id_transaction_type(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        transaction_type: &common_enums::TransactionType,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<routing_storage::RoutingProfileMetadata>>;
}

#[async_trait::async_trait]
impl RoutingAlgorithmInterface for Store {
    #[instrument(skip_all)]
    async fn insert_routing_algorithm(
        &self,
        routing_algorithm: routing_storage::RoutingAlgorithm,
    ) -> StorageResult<routing_storage::RoutingAlgorithm> {
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
    ) -> StorageResult<routing_storage::RoutingAlgorithm> {
        let conn = connection::pg_connection_write(self).await?;
        routing_storage::RoutingAlgorithm::find_by_algorithm_id_profile_id(
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
    ) -> StorageResult<routing_storage::RoutingAlgorithm> {
        let conn = connection::pg_connection_write(self).await?;
        routing_storage::RoutingAlgorithm::find_by_algorithm_id_merchant_id(
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
    ) -> StorageResult<routing_storage::RoutingProfileMetadata> {
        let conn = connection::pg_connection_write(self).await?;
        routing_storage::RoutingAlgorithm::find_metadata_by_algorithm_id_profile_id(
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
    ) -> StorageResult<Vec<routing_storage::RoutingProfileMetadata>> {
        let conn = connection::pg_connection_write(self).await?;
        routing_storage::RoutingAlgorithm::list_metadata_by_profile_id(
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
    ) -> StorageResult<Vec<routing_storage::RoutingProfileMetadata>> {
        let conn = connection::pg_connection_write(self).await?;
        routing_storage::RoutingAlgorithm::list_metadata_by_merchant_id(
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
    ) -> StorageResult<Vec<routing_storage::RoutingProfileMetadata>> {
        let conn = connection::pg_connection_write(self).await?;
        routing_storage::RoutingAlgorithm::list_metadata_by_merchant_id_transaction_type(
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

#[async_trait::async_trait]
impl RoutingAlgorithmInterface for MockDb {
    async fn insert_routing_algorithm(
        &self,
        _routing_algorithm: routing_storage::RoutingAlgorithm,
    ) -> StorageResult<routing_storage::RoutingAlgorithm> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_routing_algorithm_by_profile_id_algorithm_id(
        &self,
        _profile_id: &common_utils::id_type::ProfileId,
        _algorithm_id: &common_utils::id_type::RoutingId,
    ) -> StorageResult<routing_storage::RoutingAlgorithm> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_routing_algorithm_by_algorithm_id_merchant_id(
        &self,
        _algorithm_id: &common_utils::id_type::RoutingId,
        _merchant_id: &common_utils::id_type::MerchantId,
    ) -> StorageResult<routing_storage::RoutingAlgorithm> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_routing_algorithm_metadata_by_algorithm_id_profile_id(
        &self,
        _algorithm_id: &common_utils::id_type::RoutingId,
        _profile_id: &common_utils::id_type::ProfileId,
    ) -> StorageResult<routing_storage::RoutingProfileMetadata> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn list_routing_algorithm_metadata_by_profile_id(
        &self,
        _profile_id: &common_utils::id_type::ProfileId,
        _limit: i64,
        _offset: i64,
    ) -> StorageResult<Vec<routing_storage::RoutingProfileMetadata>> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn list_routing_algorithm_metadata_by_merchant_id(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        _limit: i64,
        _offset: i64,
    ) -> StorageResult<Vec<routing_storage::RoutingProfileMetadata>> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn list_routing_algorithm_metadata_by_merchant_id_transaction_type(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        _transaction_type: &common_enums::TransactionType,
        _limit: i64,
        _offset: i64,
    ) -> StorageResult<Vec<routing_storage::RoutingProfileMetadata>> {
        Err(errors::StorageError::MockDbError)?
    }
}
