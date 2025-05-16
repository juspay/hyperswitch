use diesel_models::dynamic_routing_stats; // For types used in KafkaStore impl
pub use hyperswitch_domain_models::db::dynamic_routing_stats::DynamicRoutingStatsInterface; // Made public
use router_env::{instrument, tracing};

use crate::{
    core::errors::{self, CustomResult},
    db::kafka_store::KafkaStore,
};

#[async_trait::async_trait]
impl DynamicRoutingStatsInterface for KafkaStore {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn insert_dynamic_routing_stat_entry(
        &self,
        dynamic_routing_stat: dynamic_routing_stats::DynamicRoutingStatsNew,
    ) -> CustomResult<dynamic_routing_stats::DynamicRoutingStats, Self::Error> {
        self.diesel_store
            .insert_dynamic_routing_stat_entry(dynamic_routing_stat)
            .await
    }

    async fn find_dynamic_routing_stats_optional_by_attempt_id_merchant_id(
        &self,
        attempt_id: String,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Option<dynamic_routing_stats::DynamicRoutingStats>, Self::Error> {
        self.diesel_store
            .find_dynamic_routing_stats_optional_by_attempt_id_merchant_id(attempt_id, merchant_id)
            .await
    }

    async fn update_dynamic_routing_stats(
        &self,
        attempt_id: String,
        merchant_id: &common_utils::id_type::MerchantId,
        data: dynamic_routing_stats::DynamicRoutingStatsUpdate,
    ) -> CustomResult<dynamic_routing_stats::DynamicRoutingStats, Self::Error> {
        self.diesel_store
            .update_dynamic_routing_stats(attempt_id, merchant_id, data)
            .await
    }
}
