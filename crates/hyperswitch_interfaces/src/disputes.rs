//! Disputes interface

use time::PrimitiveDateTime;

/// struct DisputePayload
#[derive(Default, Debug)]
pub struct DisputePayload {
    /// amount
    pub amount: String,
    /// currency
    pub currency: common_enums::enums::Currency,
    /// dispute_stage
    pub dispute_stage: common_enums::enums::DisputeStage,
    /// connector_status
    pub connector_status: String,
    /// connector_dispute_id
    pub connector_dispute_id: String,
    /// connector_reason
    pub connector_reason: Option<String>,
    /// connector_reason_code
    pub connector_reason_code: Option<String>,
    /// challenge_required_by
    pub challenge_required_by: Option<PrimitiveDateTime>,
    /// created_at
    pub created_at: Option<PrimitiveDateTime>,
    /// updated_at
    pub updated_at: Option<PrimitiveDateTime>,
}
