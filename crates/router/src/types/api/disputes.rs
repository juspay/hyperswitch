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

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct DisputeEvidence {
    pub cancellation_policy: Option<String>,
    pub customer_communication: Option<String>,
    pub customer_signature: Option<String>,
    pub receipt: Option<String>,
    pub refund_policy: Option<String>,
    pub service_documentation: Option<String>,
    pub shipping_documentation: Option<String>,
    pub invoice_showing_distinct_transactions: Option<String>,
    pub recurring_transaction_agreement: Option<String>,
    pub uncategorized_file: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttachEvidenceRequest {
    pub create_file_request: types::api::CreateFileRequest,
    pub evidence_type: EvidenceType,
}

#[derive(Debug, serde::Deserialize, strum::Display, strum::EnumString, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum EvidenceType {
    CancellationPolicy,
    CustomerCommunication,
    CustomerSignature,
    Receipt,
    RefundPolicy,
    ServiceDocumentation,
    ShippingDocumentation,
    InvoiceShowingDistinctTransactions,
    RecurringTransactionAgreement,
    UncategorizedFile,
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
