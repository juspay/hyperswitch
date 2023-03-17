use masking::{Deserialize, Serialize};

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

#[derive(Default, Clone, Debug, Serialize)]
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
    pub recieved_at: String,
}
