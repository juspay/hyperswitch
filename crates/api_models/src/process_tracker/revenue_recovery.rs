use common_utils::id_type;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

use crate::enums;
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RevenueRecoveryResponse {
    pub id: String,
    pub name: Option<String>,
    pub schedule_time_for_payment: Option<PrimitiveDateTime>,
    pub schedule_time_for_psync: Option<PrimitiveDateTime>,
    #[schema(value_type = ProcessTrackerStatus, example = "finish")]
    pub status: enums::ProcessTrackerStatus,
    pub business_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RevenueRecoveryId {
    pub revenue_recovery_id: id_type::GlobalPaymentId,
}
