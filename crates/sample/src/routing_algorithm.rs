use diesel_models::routing_algorithm as routing_storage;
// use error_stack::report;
// use router_env::{instrument, tracing};
// use storage_impl::mock_db::MockDb;

// use crate::{
//     connection,
//     core::errors::{self, CustomResult},
//     services::Store,
// };

// use hyperswitch_domain_models::errors;
use common_utils::errors::CustomResult;

#[async_trait::async_trait]
#[allow(dead_code)]
pub trait RoutingAlgorithmInterface {
    type Error;
    async fn insert_routing_algorithm(
        &self,
        routing_algorithm: routing_storage::RoutingAlgorithm,
    ) -> CustomResult<routing_storage::RoutingAlgorithm, Self::Error>;

    async fn find_routing_algorithm_by_profile_id_algorithm_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        algorithm_id: &common_utils::id_type::RoutingId,
    ) -> CustomResult<routing_storage::RoutingAlgorithm, Self::Error>;

    async fn find_routing_algorithm_by_algorithm_id_merchant_id(
        &self,
        algorithm_id: &common_utils::id_type::RoutingId,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<routing_storage::RoutingAlgorithm, Self::Error>;

    async fn find_routing_algorithm_metadata_by_algorithm_id_profile_id(
        &self,
        algorithm_id: &common_utils::id_type::RoutingId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<routing_storage::RoutingProfileMetadata, Self::Error>;

    async fn list_routing_algorithm_metadata_by_profile_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<routing_storage::RoutingProfileMetadata>, Self::Error>;

    async fn list_routing_algorithm_metadata_by_merchant_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<routing_storage::RoutingProfileMetadata>, Self::Error>;

    async fn list_routing_algorithm_metadata_by_merchant_id_transaction_type(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        transaction_type: &common_enums::TransactionType,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<routing_storage::RoutingProfileMetadata>, Self::Error>;
}

// #[async_trait::async_trait]
// impl RoutingAlgorithmInterface for MockDb {
//     async fn insert_routing_algorithm(
//         &self,
//         _routing_algorithm: routing_storage::RoutingAlgorithm,
//     ) -> StorageResult<routing_storage::RoutingAlgorithm> {
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn find_routing_algorithm_by_profile_id_algorithm_id(
//         &self,
//         _profile_id: &common_utils::id_type::ProfileId,
//         _algorithm_id: &common_utils::id_type::RoutingId,
//     ) -> StorageResult<routing_storage::RoutingAlgorithm> {
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn find_routing_algorithm_by_algorithm_id_merchant_id(
//         &self,
//         _algorithm_id: &common_utils::id_type::RoutingId,
//         _merchant_id: &common_utils::id_type::MerchantId,
//     ) -> StorageResult<routing_storage::RoutingAlgorithm> {
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn find_routing_algorithm_metadata_by_algorithm_id_profile_id(
//         &self,
//         _algorithm_id: &common_utils::id_type::RoutingId,
//         _profile_id: &common_utils::id_type::ProfileId,
//     ) -> StorageResult<routing_storage::RoutingProfileMetadata> {
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn list_routing_algorithm_metadata_by_profile_id(
//         &self,
//         _profile_id: &common_utils::id_type::ProfileId,
//         _limit: i64,
//         _offset: i64,
//     ) -> StorageResult<Vec<routing_storage::RoutingProfileMetadata>> {
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn list_routing_algorithm_metadata_by_merchant_id(
//         &self,
//         _merchant_id: &common_utils::id_type::MerchantId,
//         _limit: i64,
//         _offset: i64,
//     ) -> StorageResult<Vec<routing_storage::RoutingProfileMetadata>> {
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn list_routing_algorithm_metadata_by_merchant_id_transaction_type(
//         &self,
//         _merchant_id: &common_utils::id_type::MerchantId,
//         _transaction_type: &common_enums::TransactionType,
//         _limit: i64,
//         _offset: i64,
//     ) -> StorageResult<Vec<routing_storage::RoutingProfileMetadata>> {
//         Err(errors::StorageError::MockDbError)?
//     }
// }
