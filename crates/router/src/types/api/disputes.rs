use masking::{Deserialize, Serialize};

use crate::{services, types};

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

#[derive(Debug, Clone)]
pub struct Defend;

pub trait DefendDispute:
    services::ConnectorIntegration<
    Defend,
    types::DefendDisputeRequestData,
    types::DefendDisputeResponse,
>
{
}

pub trait Dispute: super::ConnectorCommon + AcceptDispute + SubmitEvidence + DefendDispute {}
