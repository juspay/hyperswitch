use common_utils::id_type;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::enums;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueRecoveryResponse {
    pub id: String,
    pub name: Option<String>,
    pub schedule_time_for_payment: Option<PrimitiveDateTime>,
    pub schedule_time_for_psync: Option<PrimitiveDateTime>,
    pub status: enums::ProcessTrackerStatus,
    pub business_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueRecoveryId {
    pub revenue_recovery_id: id_type::GlobalPaymentId,
}
