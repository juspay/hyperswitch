use crate::connection;
use common_utils::{errors::CustomResult, id_type};
pub use diesel_models::{enums as common_enums, routing_algorithm as storage_models};
use error_stack::report;
use hyperswitch_domain_models::db::routing_algorithm::RoutingAlgorithmInterface;
use router_env::{instrument, tracing};

use super::MockDb;
use crate::{errors, kv_router_store::KVRouterStore, DatabaseStore, RouterStore};

pub type StorageResult<T> = CustomResult<T, errors::StorageError>;

#[async_trait::async_trait]
impl<T: DatabaseStore> RoutingAlgorithmInterface for KVRouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn insert_routing_algorithm(
        &self,
        routing_algorithm: storage_models::RoutingAlgorithm,
    ) -> StorageResult<storage_models::RoutingAlgorithm> {
        let conn = connection::pg_connection_write(self).await?;
        routing_algorithm
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_routing_algorithm_by_profile_id_algorithm_id(
        &self,
        profile_id: &id_type::ProfileId,
        algorithm_id: &id_type::RoutingId,
    ) -> StorageResult<storage_models::RoutingAlgorithm> {
        let conn = connection::pg_connection_write(self).await?;
        storage_models::RoutingAlgorithm::find_by_algorithm_id_profile_id(
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
        algorithm_id: &id_type::RoutingId,
        merchant_id: &id_type::MerchantId,
    ) -> StorageResult<storage_models::RoutingAlgorithm> {
        let conn = connection::pg_connection_write(self).await?;
        storage_models::RoutingAlgorithm::find_by_algorithm_id_merchant_id(
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
        algorithm_id: &id_type::RoutingId,
        profile_id: &id_type::ProfileId,
    ) -> StorageResult<storage_models::RoutingProfileMetadata> {
        let conn = connection::pg_connection_write(self).await?;
        storage_models::RoutingAlgorithm::find_metadata_by_algorithm_id_profile_id(
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
        profile_id: &id_type::ProfileId,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<storage_models::RoutingProfileMetadata>> {
        let conn = connection::pg_connection_write(self).await?;
        storage_models::RoutingAlgorithm::list_metadata_by_profile_id(
            &conn, profile_id, limit, offset,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }
    #[instrument(skip_all)]
    async fn list_routing_algorithm_metadata_by_merchant_id(
        &self,
        merchant_id: &id_type::MerchantId,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<storage_models::RoutingProfileMetadata>> {
        let conn = connection::pg_connection_write(self).await?;
        storage_models::RoutingAlgorithm::list_metadata_by_merchant_id(
            &conn,
            merchant_id,
            limit,
            offset,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }
    #[instrument(skip_all)]
    async fn list_routing_algorithm_metadata_by_merchant_id_transaction_type(
        &self,
        merchant_id: &id_type::MerchantId,
        transaction_type: &common_enums::TransactionType,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<storage_models::RoutingProfileMetadata>> {
        let conn = connection::pg_connection_write(self).await?;
        storage_models::RoutingAlgorithm::list_metadata_by_merchant_id_transaction_type(
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
impl<T: DatabaseStore> RoutingAlgorithmInterface for RouterStore<T> {
    type Error = errors::StorageError;
    #[instrument(skip_all)]
    async fn insert_routing_algorithm(
        &self,
        routing_algorithm: storage_models::RoutingAlgorithm,
    ) -> StorageResult<storage_models::RoutingAlgorithm> {
        let conn = connection::pg_connection_write(self).await?;
        routing_algorithm
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_routing_algorithm_by_profile_id_algorithm_id(
        &self,
        profile_id: &id_type::ProfileId,
        algorithm_id: &id_type::RoutingId,
    ) -> StorageResult<storage_models::RoutingAlgorithm> {
        let conn = connection::pg_connection_write(self).await?;
        storage_models::RoutingAlgorithm::find_by_algorithm_id_profile_id(
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
        algorithm_id: &id_type::RoutingId,
        merchant_id: &id_type::MerchantId,
    ) -> StorageResult<storage_models::RoutingAlgorithm> {
        let conn = connection::pg_connection_write(self).await?;
        storage_models::RoutingAlgorithm::find_by_algorithm_id_merchant_id(
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
        algorithm_id: &id_type::RoutingId,
        profile_id: &id_type::ProfileId,
    ) -> StorageResult<storage_models::RoutingProfileMetadata> {
        let conn = connection::pg_connection_write(self).await?;
        storage_models::RoutingAlgorithm::find_metadata_by_algorithm_id_profile_id(
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
        profile_id: &id_type::ProfileId,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<storage_models::RoutingProfileMetadata>> {
        let conn = connection::pg_connection_write(self).await?;
        storage_models::RoutingAlgorithm::list_metadata_by_profile_id(
            &conn, profile_id, limit, offset,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_routing_algorithm_metadata_by_merchant_id(
        &self,
        merchant_id: &id_type::MerchantId,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<storage_models::RoutingProfileMetadata>> {
        let conn = connection::pg_connection_write(self).await?;
        storage_models::RoutingAlgorithm::list_metadata_by_merchant_id(
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
        merchant_id: &id_type::MerchantId,
        transaction_type: &common_enums::TransactionType,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<storage_models::RoutingProfileMetadata>> {
        let conn = connection::pg_connection_write(self).await?;
        storage_models::RoutingAlgorithm::list_metadata_by_merchant_id_transaction_type(
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
    type Error = errors::StorageError;
    async fn insert_routing_algorithm(
        &self,
        _routing_algorithm: storage_models::RoutingAlgorithm,
    ) -> StorageResult<storage_models::RoutingAlgorithm> {
        Err(report!(errors::StorageError::MockDbError))
    }

    async fn find_routing_algorithm_by_profile_id_algorithm_id(
        &self,
        _profile_id: &id_type::ProfileId,
        _algorithm_id: &id_type::RoutingId,
    ) -> StorageResult<storage_models::RoutingAlgorithm> {
        Err(report!(errors::StorageError::MockDbError))
    }

    async fn find_routing_algorithm_by_algorithm_id_merchant_id(
        &self,
        _algorithm_id: &id_type::RoutingId,
        _merchant_id: &id_type::MerchantId,
    ) -> StorageResult<storage_models::RoutingAlgorithm> {
        Err(report!(errors::StorageError::MockDbError))
    }

    async fn find_routing_algorithm_metadata_by_algorithm_id_profile_id(
        &self,
        _algorithm_id: &id_type::RoutingId,
        _profile_id: &id_type::ProfileId,
    ) -> StorageResult<storage_models::RoutingProfileMetadata> {
        Err(report!(errors::StorageError::MockDbError))
    }

    async fn list_routing_algorithm_metadata_by_profile_id(
        &self,
        _profile_id: &id_type::ProfileId,
        _limit: i64,
        _offset: i64,
    ) -> StorageResult<Vec<storage_models::RoutingProfileMetadata>> {
        Err(report!(errors::StorageError::MockDbError))
    }

    async fn list_routing_algorithm_metadata_by_merchant_id(
        &self,
        _merchant_id: &id_type::MerchantId,
        _limit: i64,
        _offset: i64,
    ) -> StorageResult<Vec<storage_models::RoutingProfileMetadata>> {
        Err(report!(errors::StorageError::MockDbError))
    }

    async fn list_routing_algorithm_metadata_by_merchant_id_transaction_type(
        &self,
        _merchant_id: &id_type::MerchantId,
        _transaction_type: &common_enums::TransactionType,
        _limit: i64,
        _offset: i64,
    ) -> StorageResult<Vec<storage_models::RoutingProfileMetadata>> {
        Err(report!(errors::StorageError::MockDbError))
    }
}
