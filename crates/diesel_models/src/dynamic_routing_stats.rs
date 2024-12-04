use diesel::{Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};

use crate::schema::dynamic_routing_stats;

#[derive(Clone, Debug, Eq, Insertable, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = dynamic_routing_stats)]
pub struct DynamicRoutingStatsNew {
    pub payment_id: common_utils::id_type::PaymentId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub profile_id: common_utils::id_type::ProfileId,
    pub success_based_routing_connector: String,
    pub payment_connector: String,
    pub currency: Option<String>,
    pub payment_method: Option<String>,
    pub capture_method: Option<String>,
    pub authentication_type: Option<String>,
    pub payment_status: String,
    pub conclusive_classification: common_enums::SuccessBasedRoutingConclusiveState,
    pub created_at: time::PrimitiveDateTime,
}

#[derive(
    Clone, Debug, Eq, PartialEq, Queryable, Selectable, Deserialize, Serialize, Insertable,
)]
#[diesel(table_name = dynamic_routing_stats, primary_key(payment_id), check_for_backend(diesel::pg::Pg))]
pub struct DynamicRoutingStats {
    pub payment_id: common_utils::id_type::PaymentId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub profile_id: common_utils::id_type::ProfileId,
    pub success_based_routing_connector: String,
    pub payment_connector: String,
    pub currency: Option<String>,
    pub payment_method: Option<String>,
    pub capture_method: Option<String>,
    pub authentication_type: Option<String>,
    pub payment_status: String,
    pub conclusive_classification: common_enums::SuccessBasedRoutingConclusiveState,
    pub created_at: time::PrimitiveDateTime,
}
