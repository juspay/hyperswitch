//! Disputes interface
use common_utils::types::StringMinorUnit;
use hyperswitch_domain_models::router_response_types::DisputeSyncResponse;
use time::PrimitiveDateTime;

/// struct DisputePayload
#[derive(Default, Debug)]
pub struct DisputePayload {
    /// amount
    pub amount: StringMinorUnit,
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

impl From<DisputeSyncResponse> for DisputePayload {
    fn from(dispute_sync_data: DisputeSyncResponse) -> Self {
        Self {
            amount: dispute_sync_data.amount,
            currency: dispute_sync_data.currency,
            dispute_stage: dispute_sync_data.dispute_stage,
            connector_status: dispute_sync_data.connector_status,
            connector_dispute_id: dispute_sync_data.connector_dispute_id,
            connector_reason: dispute_sync_data.connector_reason,
            connector_reason_code: dispute_sync_data.connector_reason_code,
            challenge_required_by: dispute_sync_data.challenge_required_by,
            created_at: dispute_sync_data.created_at,
            updated_at: dispute_sync_data.updated_at,
        }
    }
}
