use diesel::{Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};

use crate::schema::dynamic_routing_stats;

#[derive(Clone, Debug, Eq, Insertable, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = dynamic_routing_stats)]
pub struct DynamicRoutingStatsNew {
    pub payment_id: String,
    pub tenant_id: String,
    pub merchant_id: String,
    pub profile_id: String,
    pub success_based_routing_connector: Option<String>,
    pub payment_connector: Option<String>,
    pub currency: Option<String>,
    pub payment_method: Option<String>,
    pub capture_method: Option<String>,
    pub authentication_type: Option<String>,
    pub payment_status: Option<String>,
    pub conclusive_classification: Option<String>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
}

#[derive(
    Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Selectable, Deserialize, Serialize,
)]
#[diesel(table_name = dynamic_routing_stats, primary_key(payment_id), check_for_backend(diesel::pg::Pg))]
pub struct DynamicRoutingStats {
    pub payment_id: String,
    pub tenant_id: String,
    pub merchant_id: String,
    pub profile_id: String,
    pub success_based_routing_connector: Option<String>,
    pub payment_connector: Option<String>,
    pub currency: Option<String>,
    pub payment_method: Option<String>,
    pub capture_method: Option<String>,
    pub authentication_type: Option<String>,
    pub payment_status: Option<String>,
    pub conclusive_classification: Option<common_enums::SuccessBasedRoutingConclusiveState>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
}
