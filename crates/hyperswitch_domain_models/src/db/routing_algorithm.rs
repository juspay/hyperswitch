use common_utils::{errors::CustomResult, id_type};
use diesel_models::{enums as common_enums, routing_algorithm as storage_models};

#[async_trait::async_trait]
pub trait RoutingAlgorithmInterface {
    type Error;
    async fn insert_routing_algorithm(
        &self,
        routing_algorithm: storage_models::RoutingAlgorithm,
    ) -> CustomResult<storage_models::RoutingAlgorithm, Self::Error>;

    async fn find_routing_algorithm_by_profile_id_algorithm_id(
        &self,
        profile_id: &id_type::ProfileId,
        algorithm_id: &id_type::RoutingId,
    ) -> CustomResult<storage_models::RoutingAlgorithm, Self::Error>;

    async fn find_routing_algorithm_by_algorithm_id_merchant_id(
        &self,
        algorithm_id: &id_type::RoutingId,
        merchant_id: &id_type::MerchantId,
    ) -> CustomResult<storage_models::RoutingAlgorithm, Self::Error>;

    async fn find_routing_algorithm_metadata_by_algorithm_id_profile_id(
        &self,
        algorithm_id: &id_type::RoutingId,
        profile_id: &id_type::ProfileId,
    ) -> CustomResult<storage_models::RoutingProfileMetadata, Self::Error>;

    async fn list_routing_algorithm_metadata_by_profile_id(
        &self,
        profile_id: &id_type::ProfileId,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage_models::RoutingProfileMetadata>, Self::Error>;

    async fn list_routing_algorithm_metadata_by_merchant_id(
        &self,
        merchant_id: &id_type::MerchantId,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage_models::RoutingProfileMetadata>, Self::Error>;

    async fn list_routing_algorithm_metadata_by_merchant_id_transaction_type(
        &self,
        merchant_id: &id_type::MerchantId,
        transaction_type: &common_enums::TransactionType,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage_models::RoutingProfileMetadata>, Self::Error>;
}
