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
        /// Asynchronously inserts a routing algorithm into the storage.
    ///
    /// # Arguments
    ///
    /// * `routing_algorithm` - The routing algorithm to be inserted into the storage.
    ///
    /// # Returns
    ///
    /// Returns a `StorageResult` containing the inserted routing algorithm if successful.
    ///
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

        /// Finds a routing algorithm by the given profile ID and algorithm ID in the storage.
    /// Returns a Result containing the found RoutingAlgorithm if successful, or an error if the algorithm
    /// is not found or if there was an issue retrieving it from the storage.
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

        /// Asynchronously finds a routing algorithm by its algorithm ID and merchant ID from the storage.
    ///
    /// # Arguments
    ///
    /// * `algorithm_id` - The ID of the routing algorithm to find.
    /// * `merchant_id` - The ID of the merchant associated with the routing algorithm.
    ///
    /// # Returns
    ///
    /// A `StorageResult` containing the found `RoutingAlgorithm`, or an error if the algorithm was not found.
    ///
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

        /// Asynchronously finds routing algorithm metadata by algorithm ID and profile ID.
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

        /// Asynchronously retrieves a list of routing algorithm metadata associated with a given profile ID, with optional limits and offsets. 
    ///
    /// # Arguments
    ///
    /// * `profile_id` - The ID of the profile for which to retrieve routing algorithm metadata.
    /// * `limit` - The maximum number of records to retrieve.
    /// * `offset` - The number of records to skip before starting to return records.
    ///
    /// # Returns
    ///
    /// A `StorageResult` containing a vector of `RoutingAlgorithmMetadata` if successful, or an error if the operation fails.
    ///
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

        /// Retrieves a list of routing algorithm metadata associated with a specific merchant ID, with the option to limit the number of results and specify the offset.
    /// 
    /// # Arguments
    /// 
    /// * `merchant_id` - The ID of the merchant for which to retrieve routing algorithm metadata
    /// * `limit` - The maximum number of results to return
    /// * `offset` - The offset for paginating through the results
    /// 
    /// # Returns
    /// 
    /// A `Vec` of `RoutingProfileMetadata` objects representing the routing algorithm metadata associated with the specified merchant ID
    /// 
    /// # Errors
    /// 
    /// Returns a `StorageError` if the operation fails
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
        /// Asynchronously inserts a routing algorithm into the storage.
    ///
    /// # Arguments
    ///
    /// * `_routing_algorithm` - The routing algorithm to be inserted into the storage
    ///
    /// # Returns
    ///
    /// Returns a `StorageResult` containing the inserted routing algorithm if successful, otherwise returns a `StorageError` indicating a mock database error.
    ///
    async fn insert_routing_algorithm(
        &self,
        _routing_algorithm: routing_storage::RoutingAlgorithm,
    ) -> StorageResult<routing_storage::RoutingAlgorithm> {
        Err(errors::StorageError::MockDbError)?
    }

        /// Asynchronously finds a routing algorithm by the given profile ID and algorithm ID.
    ///
    /// # Arguments
    ///
    /// * `_profile_id` - The profile ID to search for.
    /// * `_algorithm_id` - The algorithm ID to search for.
    ///
    /// # Returns
    ///
    /// A `StorageResult` containing the found `RoutingAlgorithm` if successful, or a `StorageError` if an error occurs.
    ///
    /// # Errors
    ///
    /// Returns a `MockDbError` if the operation fails.
    ///
    async fn find_routing_algorithm_by_profile_id_algorithm_id(
        &self,
        _profile_id: &str,
        _algorithm_id: &str,
    ) -> StorageResult<routing_storage::RoutingAlgorithm> {
        Err(errors::StorageError::MockDbError)?
    }

        /// Asynchronously finds a routing algorithm by its algorithm ID and merchant ID from the storage.
    /// 
    /// # Arguments
    /// * `_algorithm_id` - The ID of the routing algorithm to be found.
    /// * `_merchant_id` - The ID of the merchant associated with the routing algorithm.
    /// 
    /// # Returns
    /// The result of the operation, containing the found routing algorithm on success, or a StorageError on failure.
    /// 
    async fn find_routing_algorithm_by_algorithm_id_merchant_id(
        &self,
        _algorithm_id: &str,
        _merchant_id: &str,
    ) -> StorageResult<routing_storage::RoutingAlgorithm> {
        Err(errors::StorageError::MockDbError)?
    }

        /// Asynchronously finds the routing algorithm metadata by the provided algorithm ID and profile ID.
    ///
    /// # Arguments
    ///
    /// * `_algorithm_id` - A reference to a string representing the algorithm ID.
    /// * `_profile_id` - A reference to a string representing the profile ID.
    ///
    /// # Returns
    ///
    /// A `StorageResult` containing the `RoutingProfileMetadata` if found, otherwise a `StorageError::MockDbError`.
    ///
    async fn find_routing_algorithm_metadata_by_algorithm_id_profile_id(
        &self,
        _algorithm_id: &str,
        _profile_id: &str,
    ) -> StorageResult<routing_storage::RoutingProfileMetadata> {
        Err(errors::StorageError::MockDbError)?
    }

        /// Asynchronously retrieves a list of routing algorithm metadata by profile ID with a specified limit and offset.
    ///
    /// # Arguments
    ///
    /// * `_profile_id` - A reference to a string representing the profile ID for which to retrieve the routing algorithm metadata.
    /// * `_limit` - An i64 representing the maximum number of items to retrieve.
    /// * `_offset` - An i64 representing the number of items to skip before starting to return items.
    ///
    /// # Returns
    ///
    /// A `StorageResult` containing a vector of `RoutingAlgorithmMetadata` if successful, otherwise a `StorageError` indicating the failure reason.
    ///
    async fn list_routing_algorithm_metadata_by_profile_id(
        &self,
        _profile_id: &str,
        _limit: i64,
        _offset: i64,
    ) -> StorageResult<Vec<routing_storage::RoutingAlgorithmMetadata>> {
        Err(errors::StorageError::MockDbError)?
    }

        /// Retrieves a list of routing algorithm metadata associated with a specific merchant ID from the storage.
    ///
    /// # Arguments
    ///
    /// * `_merchant_id` - A string slice representing the merchant ID for which the routing algorithm metadata should be retrieved.
    /// * `_limit` - An i64 representing the maximum number of records to retrieve.
    /// * `_offset` - An i64 representing the number of records to skip before starting to return records.
    ///
    /// # Returns
    ///
    /// A `StorageResult` containing a vector of `RoutingProfileMetadata` if successful, otherwise a `StorageError`.
    ///
    /// # Errors
    ///
    /// Returns a `StorageError::MockDbError` if the operation fails.
    ///
    async fn list_routing_algorithm_metadata_by_merchant_id(
        &self,
        _merchant_id: &str,
        _limit: i64,
        _offset: i64,
    ) -> StorageResult<Vec<routing_storage::RoutingProfileMetadata>> {
        Err(errors::StorageError::MockDbError)?
    }
}
