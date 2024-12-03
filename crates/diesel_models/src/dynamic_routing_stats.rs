use diesel::{Insertable, Queryable, Selectable};
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
    pub conclusive_classification: Option<common_enums::SuccessBasedRoutingConclusiveState>,
    pub created_at: time::PrimitiveDateTime,
}

#[derive(
    Clone, Debug, Eq, PartialEq, Queryable, Selectable, Deserialize, Serialize, Insertable,
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
}

impl DynamicRoutingStatsNew {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: String,
        payment_id: String,
        merchant_id: String,
        profile_id: String,
        success_based_routing_connector: Option<String>,
        payment_connector: Option<String>,
        currency: Option<String>,
        payment_method: Option<String>,
        capture_method: Option<String>,
        authentication_type: Option<String>,
        payment_status: Option<String>,
        conclusive_classification: Option<common_enums::SuccessBasedRoutingConclusiveState>,
        created_at: time::PrimitiveDateTime,
    ) -> Self {
        Self {
            payment_id,
            tenant_id,
            merchant_id,
            profile_id,
            success_based_routing_connector,
            payment_connector,
            currency,
            payment_method,
            capture_method,
            authentication_type,
            payment_status,
            conclusive_classification,
            created_at,
        }
    }
}
