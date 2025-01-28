use diesel::{Insertable, Queryable, Selectable};

use crate::schema::dynamic_routing_stats;

#[derive(Clone, Debug, Eq, Insertable, PartialEq)]
#[diesel(table_name = dynamic_routing_stats)]
pub struct DynamicRoutingStatsNew {
    pub payment_id: common_utils::id_type::PaymentId,
    pub attempt_id: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub profile_id: common_utils::id_type::ProfileId,
    pub amount: common_utils::types::MinorUnit,
    pub success_based_routing_connector: String,
    pub payment_connector: String,
    pub currency: Option<common_enums::Currency>,
    pub payment_method: Option<common_enums::PaymentMethod>,
    pub capture_method: Option<common_enums::CaptureMethod>,
    pub authentication_type: Option<common_enums::AuthenticationType>,
    pub payment_status: common_enums::AttemptStatus,
    pub conclusive_classification: common_enums::SuccessBasedRoutingConclusiveState,
    pub created_at: time::PrimitiveDateTime,
    pub payment_method_type: Option<common_enums::PaymentMethodType>,
    pub global_success_based_connector: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Queryable, Selectable, Insertable)]
#[diesel(table_name = dynamic_routing_stats, primary_key(payment_id), check_for_backend(diesel::pg::Pg))]
pub struct DynamicRoutingStats {
    pub payment_id: common_utils::id_type::PaymentId,
    pub attempt_id: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub profile_id: common_utils::id_type::ProfileId,
    pub amount: common_utils::types::MinorUnit,
    pub success_based_routing_connector: String,
    pub payment_connector: String,
    pub currency: Option<common_enums::Currency>,
    pub payment_method: Option<common_enums::PaymentMethod>,
    pub capture_method: Option<common_enums::CaptureMethod>,
    pub authentication_type: Option<common_enums::AuthenticationType>,
    pub payment_status: common_enums::AttemptStatus,
    pub conclusive_classification: common_enums::SuccessBasedRoutingConclusiveState,
    pub created_at: time::PrimitiveDateTime,
    pub payment_method_type: Option<common_enums::PaymentMethodType>,
    pub global_success_based_connector: Option<String>,
}
