use masking::Serialize;
use utoipa::ToSchema;

use super::enums::{DisputeStage, DisputeStatus};

#[derive(Default, Clone, Debug, Serialize, ToSchema)]
pub struct DisputeResponse {
    /// The identifier for dispute
    pub dispute_id: String,
    /// The identifier for payment_intent
    pub payment_id: String,
    /// The identifier for payment_attempt
    pub attempt_id: String,
    /// The dispute amount
    pub amount: String,
    /// The three-letter ISO currency code
    pub currency: String,
    /// Stage of the dispute
    pub dispute_stage: DisputeStage,
    /// Status of the dispute
    pub dispute_status: DisputeStatus,
    /// Status of the dispute sent by connector
    pub connector_status: String,
    /// Dispute id sent by connector
    pub connector_dispute_id: String,
    /// Reason of dispute sent by connector
    pub connector_reason: Option<String>,
    /// Reason code of dispute sent by connector
    pub connector_reason_code: Option<String>,
    /// Evidence deadline of dispute sent by connector
    pub challenge_required_by: Option<String>,
    /// Dispute created time sent by connector
    pub created_at: Option<String>,
    /// Dispute updated time sent by connector
    pub updated_at: Option<String>,
    /// Time at which dispute is received
    pub received_at: String,
}
