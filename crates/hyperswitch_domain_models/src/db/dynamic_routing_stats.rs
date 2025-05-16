use common_utils::errors::CustomResult;
use diesel_models::dynamic_routing_stats;

#[async_trait::async_trait]
pub trait DynamicRoutingStatsInterface {
    type Error;

    async fn insert_dynamic_routing_stat_entry(
        &self,
        dynamic_routing_stat_new: dynamic_routing_stats::DynamicRoutingStatsNew,
    ) -> CustomResult<dynamic_routing_stats::DynamicRoutingStats, Self::Error>;

    async fn find_dynamic_routing_stats_optional_by_attempt_id_merchant_id(
        &self,
        attempt_id: String,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Option<dynamic_routing_stats::DynamicRoutingStats>, Self::Error>;

    async fn update_dynamic_routing_stats(
        &self,
        attempt_id: String,
        merchant_id: &common_utils::id_type::MerchantId,
        data: dynamic_routing_stats::DynamicRoutingStatsUpdate,
    ) -> CustomResult<dynamic_routing_stats::DynamicRoutingStats, Self::Error>;
}
