use masking::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{services, types};

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct DisputeId {
    pub dispute_id: String,
}

#[derive(Default, Debug)]
pub struct DisputePayload {
    pub amount: String,
    pub currency: String,
    pub dispute_stage: api_models::enums::DisputeStage,
    pub connector_status: String,
    pub connector_dispute_id: String,
    pub connector_reason: Option<String>,
    pub connector_reason_code: Option<String>,
    pub challenge_required_by: Option<PrimitiveDateTime>,
    pub created_at: Option<PrimitiveDateTime>,
    pub updated_at: Option<PrimitiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct Accept;

pub trait AcceptDispute:
    services::ConnectorIntegration<
    Accept,
    types::AcceptDisputeRequestData,
    types::AcceptDisputeResponse,
>
{
}

#[derive(Debug, Clone)]
pub struct Evidence;

pub trait SubmitEvidence:
    services::ConnectorIntegration<
    Evidence,
    types::SubmitEvidenceRequestData,
    types::SubmitEvidenceResponse,
>
{
}

pub trait Dispute: super::ConnectorCommon + AcceptDispute + SubmitEvidence {}
