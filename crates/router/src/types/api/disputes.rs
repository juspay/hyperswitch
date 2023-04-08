use api_models::disputes;
use masking::{Deserialize, Serialize};

use crate::services;

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct DisputeId {
    pub dispute_id: String,
}

#[derive(Default, Debug, Deserialize)]
pub struct DisputePayload {
    pub amount: String,
    pub currency: String,
    pub dispute_stage: api_models::enums::DisputeStage,
    pub connector_status: String,
    pub connector_dispute_id: String,
    pub connector_reason: Option<String>,
    pub connector_reason_code: Option<String>,
    pub challenge_required_by: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct AcceptDisputeRequestData {
    pub dispute_id: String,
    pub connector_dispute_id: String,
}

#[derive(Debug, Clone)]
pub struct Accept;

pub trait AcceptDispute:
    services::ConnectorIntegration<Accept, AcceptDisputeRequestData, disputes::AcceptDisputeResponse>
{
}

pub trait Dispute: super::ConnectorCommon + AcceptDispute {}
