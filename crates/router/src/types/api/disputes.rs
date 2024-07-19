pub use hyperswitch_interfaces::{
    api::disputes::{AcceptDispute, DefendDispute, Dispute, SubmitEvidence},
    disputes::DisputePayload,
};
use masking::{Deserialize, Serialize};

use crate::types;

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct DisputeId {
    pub dispute_id: String,
}

pub use hyperswitch_domain_models::router_flow_types::dispute::{Accept, Defend, Evidence};

pub use super::disputes_v2::{AcceptDisputeV2, DefendDisputeV2, DisputeV2, SubmitEvidenceV2};

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
