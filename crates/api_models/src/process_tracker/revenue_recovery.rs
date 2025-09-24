use common_utils::id_type;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

use crate::enums;
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RevenueRecoveryResponse {
    pub id: String,
    pub name: Option<String>,
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub schedule_time_for_payment: Option<PrimitiveDateTime>,
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub schedule_time_for_psync: Option<PrimitiveDateTime>,
    #[schema(value_type = ProcessTrackerStatus, example = "finish")]
    pub status: enums::ProcessTrackerStatus,
    pub business_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RevenueRecoveryId {
    pub revenue_recovery_id: id_type::GlobalPaymentId,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RevenueRecoveryRetriggerRequest {
    /// The task we want to resume
    pub revenue_recovery_task: String,
    /// Time at which the job was scheduled at
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub schedule_time: Option<PrimitiveDateTime>,
    /// Status of The Process Tracker Task
    pub status: enums::ProcessTrackerStatus,
    /// Business Status of The Process Tracker Task
    pub business_status: String,
}
