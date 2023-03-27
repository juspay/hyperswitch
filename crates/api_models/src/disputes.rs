use masking::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

use crate::enums::{DisputeStage, DisputeStatus};

#[derive(Default, Debug, Deserialize)]
pub struct DisputePayload {
    pub amount: String,
    pub currency: String,
    pub dispute_stage: DisputeStage,
    pub connector_status: String,
    pub connector_dispute_id: String,
    pub connector_reason: Option<String>,
    pub connector_reason_code: Option<String>,
    pub challenge_required_by: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Default, Clone, Debug, Serialize, ToSchema)]
pub struct DisputeResponse {
    pub dispute_id: String,
    pub payment_id: String,
    pub amount: String,
    pub currency: String,
    pub dispute_stage: DisputeStage,
    pub dispute_status: DisputeStatus,
    pub connector_status: String,
    pub connector_dispute_id: String,
    pub connector_reason: Option<String>,
    pub connector_reason_code: Option<String>,
    pub challenge_required_by: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub received_at: String,
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DisputeListConstraints {
    /// limit on the number of objects to return
    #[schema(default = 10)]
    pub limit: Option<i64>,
    pub dispute_status: Option<DisputeStatus>,
    pub dispute_stage: Option<DisputeStage>,
    pub reason: Option<String>,
    pub connector: Option<String>,
    /// The time at which payment is created
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub received_time: Option<PrimitiveDateTime>,
    /// Time less than the dispute received time
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(
        default,
        with = "common_utils::custom_serde::iso8601::option",
        rename = "received_time.lt"
    )]
    pub received_time_lt: Option<PrimitiveDateTime>,
    /// Time greater than the dispute received time
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(
        default,
        with = "common_utils::custom_serde::iso8601::option",
        rename = "received_time.gt"
    )]
    pub received_time_gt: Option<PrimitiveDateTime>,
    /// Time less than or equals to the dispute received time
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(
        default,
        with = "common_utils::custom_serde::iso8601::option",
        rename = "received_time.lte"
    )]
    pub received_time_lte: Option<PrimitiveDateTime>,
    /// Time greater than or equals to the dispute received time
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    #[serde(rename = "received_time.gte")]
    pub received_time_gte: Option<PrimitiveDateTime>,
}
