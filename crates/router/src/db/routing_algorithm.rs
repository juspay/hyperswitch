use diesel_models::routing_algorithm as routing_storage;
use error_stack::IntoReport;
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
        profile_id: &str,
        algorithm_id: &str,
    ) -> StorageResult<routing_storage::RoutingAlgorithm>;

    async fn find_routing_algorithm_by_algorithm_id_merchant_id(
        &self,
        algorithm_id: &str,
        merchant_id: &str,
    ) -> StorageResult<routing_storage::RoutingAlgorithm>;

    async fn find_routing_algorithm_metadata_by_algorithm_id_profile_id(
        &self,
        algorithm_id: &str,
        profile_id: &str,
    ) -> StorageResult<routing_storage::RoutingProfileMetadata>;

    async fn list_routing_algorithm_metadata_by_profile_id(
        &self,
        profile_id: &str,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<routing_storage::RoutingAlgorithmMetadata>>;

    async fn list_routing_algorithm_metadata_by_merchant_id(
        &self,
        merchant_id: &str,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<routing_storage::RoutingProfileMetadata>>;
}

#[async_trait::async_trait]
impl RoutingAlgorithmInterface for Store {
    async fn insert_routing_algorithm(
        &self,
        routing_algorithm: routing_storage::RoutingAlgorithm,
    ) -> StorageResult<routing_storage::RoutingAlgorithm> {
        let conn = connection::pg_connection_write(self).await?;
        routing_algorithm
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_routing_algorithm_by_profile_id_algorithm_id(
        &self,
        profile_id: &str,
        algorithm_id: &str,
    ) -> StorageResult<routing_storage::RoutingAlgorithm> {
        let conn = connection::pg_connection_write(self).await?;
        routing_storage::RoutingAlgorithm::find_by_algorithm_id_profile_id(
            &conn,
            algorithm_id,
            profile_id,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

    async fn find_routing_algorithm_by_algorithm_id_merchant_id(
        &self,
        algorithm_id: &str,
        merchant_id: &str,
    ) -> StorageResult<routing_storage::RoutingAlgorithm> {
        let conn = connection::pg_connection_write(self).await?;
        routing_storage::RoutingAlgorithm::find_by_algorithm_id_merchant_id(
            &conn,
            algorithm_id,
            merchant_id,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

    async fn find_routing_algorithm_metadata_by_algorithm_id_profile_id(
        &self,
        algorithm_id: &str,
        profile_id: &str,
    ) -> StorageResult<routing_storage::RoutingProfileMetadata> {
        let conn = connection::pg_connection_write(self).await?;
        routing_storage::RoutingAlgorithm::find_metadata_by_algorithm_id_profile_id(
            &conn,
            algorithm_id,
            profile_id,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

    async fn list_routing_algorithm_metadata_by_profile_id(
        &self,
        profile_id: &str,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<routing_storage::RoutingAlgorithmMetadata>> {
        let conn = connection::pg_connection_write(self).await?;
        routing_storage::RoutingAlgorithm::list_metadata_by_profile_id(
            &conn, profile_id, limit, offset,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

    async fn list_routing_algorithm_metadata_by_merchant_id(
        &self,
        merchant_id: &str,
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
        .map_err(Into::into)
        .into_report()
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
        _profile_id: &str,
        _algorithm_id: &str,
    ) -> StorageResult<routing_storage::RoutingAlgorithm> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_routing_algorithm_by_algorithm_id_merchant_id(
        &self,
        _algorithm_id: &str,
        _merchant_id: &str,
    ) -> StorageResult<routing_storage::RoutingAlgorithm> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_routing_algorithm_metadata_by_algorithm_id_profile_id(
        &self,
        _algorithm_id: &str,
        _profile_id: &str,
    ) -> StorageResult<routing_storage::RoutingProfileMetadata> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn list_routing_algorithm_metadata_by_profile_id(
        &self,
        _profile_id: &str,
        _limit: i64,
        _offset: i64,
    ) -> StorageResult<Vec<routing_storage::RoutingAlgorithmMetadata>> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn list_routing_algorithm_metadata_by_merchant_id(
        &self,
        _merchant_id: &str,
        _limit: i64,
        _offset: i64,
    ) -> StorageResult<Vec<routing_storage::RoutingProfileMetadata>> {
        Err(errors::StorageError::MockDbError)?
    }
}
