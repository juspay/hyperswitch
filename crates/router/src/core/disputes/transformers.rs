use common_utils::errors::CustomResult;

use crate::{
    core::{errors, files::helpers::retrieve_file_and_provider_file_id_from_file_id},
    routes::AppState,
    types::{
        api::{self, DisputeEvidence},
        SubmitEvidenceRequestData,
    },
};

pub async fn get_evidence_request_data(
    state: &AppState,
    merchant_account: &storage_models::merchant_account::MerchantAccount,
    evidence_request: api_models::disputes::SubmitEvidenceRequest,
    dispute: &storage_models::dispute::Dispute,
) -> CustomResult<SubmitEvidenceRequestData, errors::ApiErrorResponse> {
    let (cancellation_policy, cancellation_policy_provider_file_id) =
        retrieve_file_and_provider_file_id_from_file_id(
            state,
            evidence_request.cancellation_policy,
            merchant_account,
            api::FileDataRequired::NotRequired,
        )
        .await?;
    let (customer_communication, customer_communication_provider_file_id) =
        retrieve_file_and_provider_file_id_from_file_id(
            state,
            evidence_request.customer_communication,
            merchant_account,
            api::FileDataRequired::NotRequired,
        )
        .await?;
    let (customer_signature, customer_signature_provider_file_id) =
        retrieve_file_and_provider_file_id_from_file_id(
            state,
            evidence_request.customer_signature,
            merchant_account,
            api::FileDataRequired::NotRequired,
        )
        .await?;
    let (receipt, receipt_provider_file_id) = retrieve_file_and_provider_file_id_from_file_id(
        state,
        evidence_request.receipt,
        merchant_account,
        api::FileDataRequired::NotRequired,
    )
    .await?;
    let (refund_policy, refund_policy_provider_file_id) =
        retrieve_file_and_provider_file_id_from_file_id(
            state,
            evidence_request.refund_policy,
            merchant_account,
            api::FileDataRequired::NotRequired,
        )
        .await?;
    let (service_documentation, service_documentation_provider_file_id) =
        retrieve_file_and_provider_file_id_from_file_id(
            state,
            evidence_request.service_documentation,
            merchant_account,
            api::FileDataRequired::NotRequired,
        )
        .await?;
    let (shipping_documentation, shipping_documentation_provider_file_id) =
        retrieve_file_and_provider_file_id_from_file_id(
            state,
            evidence_request.shipping_documentation,
            merchant_account,
            api::FileDataRequired::NotRequired,
        )
        .await?;
    let (
        invoice_showing_distinct_transactions,
        invoice_showing_distinct_transactions_provider_file_id,
    ) = retrieve_file_and_provider_file_id_from_file_id(
        state,
        evidence_request.invoice_showing_distinct_transactions,
        merchant_account,
        api::FileDataRequired::NotRequired,
    )
    .await?;
    let (recurring_transaction_agreement, recurring_transaction_agreement_provider_file_id) =
        retrieve_file_and_provider_file_id_from_file_id(
            state,
            evidence_request.recurring_transaction_agreement,
            merchant_account,
            api::FileDataRequired::NotRequired,
        )
        .await?;
    let (uncategorized_file, uncategorized_file_provider_file_id) =
        retrieve_file_and_provider_file_id_from_file_id(
            state,
            evidence_request.uncategorized_file,
            merchant_account,
            api::FileDataRequired::NotRequired,
        )
        .await?;
    Ok(SubmitEvidenceRequestData {
        dispute_id: dispute.dispute_id.clone(),
        connector_dispute_id: dispute.connector_dispute_id.clone(),
        access_activity_log: evidence_request.access_activity_log,
        billing_address: evidence_request.billing_address,
        cancellation_policy,
        cancellation_policy_provider_file_id,
        cancellation_policy_disclosure: evidence_request.cancellation_policy_disclosure,
        cancellation_rebuttal: evidence_request.cancellation_rebuttal,
        customer_communication,
        customer_communication_provider_file_id,
        customer_email_address: evidence_request.customer_email_address,
        customer_name: evidence_request.customer_name,
        customer_purchase_ip: evidence_request.customer_purchase_ip,
        customer_signature,
        customer_signature_provider_file_id,
        product_description: evidence_request.product_description,
        receipt,
        receipt_provider_file_id,
        refund_policy,
        refund_policy_provider_file_id,
        refund_policy_disclosure: evidence_request.refund_policy_disclosure,
        refund_refusal_explanation: evidence_request.refund_refusal_explanation,
        service_date: evidence_request.service_date,
        service_documentation,
        service_documentation_provider_file_id,
        shipping_address: evidence_request.shipping_address,
        shipping_carrier: evidence_request.shipping_carrier,
        shipping_date: evidence_request.shipping_date,
        shipping_documentation,
        shipping_documentation_provider_file_id,
        shipping_tracking_number: evidence_request.shipping_tracking_number,
        invoice_showing_distinct_transactions,
        invoice_showing_distinct_transactions_provider_file_id,
        recurring_transaction_agreement,
        recurring_transaction_agreement_provider_file_id,
        uncategorized_file,
        uncategorized_file_provider_file_id,
        uncategorized_text: evidence_request.uncategorized_text,
    })
}

pub fn update_dispute_evidence(
    dispute_evidence: DisputeEvidence,
    evidence_type: api::EvidenceType,
    file_id: String,
) -> DisputeEvidence {
    match evidence_type {
        api::EvidenceType::CancellationPolicy => DisputeEvidence {
            cancellation_policy: Some(file_id),
            ..dispute_evidence
        },
        api::EvidenceType::CustomerCommunication => DisputeEvidence {
            customer_communication: Some(file_id),
            ..dispute_evidence
        },
        api::EvidenceType::CustomerSignature => DisputeEvidence {
            customer_signature: Some(file_id),
            ..dispute_evidence
        },
        api::EvidenceType::Receipt => DisputeEvidence {
            receipt: Some(file_id),
            ..dispute_evidence
        },
        api::EvidenceType::RefundPolicy => DisputeEvidence {
            refund_policy: Some(file_id),
            ..dispute_evidence
        },
        api::EvidenceType::ServiceDocumentation => DisputeEvidence {
            service_documentation: Some(file_id),
            ..dispute_evidence
        },
        api::EvidenceType::ShippingDocumentation => DisputeEvidence {
            shipping_documentation: Some(file_id),
            ..dispute_evidence
        },
        api::EvidenceType::InvoiceShowingDistinctTransactions => DisputeEvidence {
            invoice_showing_distinct_transactions: Some(file_id),
            ..dispute_evidence
        },
        api::EvidenceType::RecurringTransactionAgreement => DisputeEvidence {
            recurring_transaction_agreement: Some(file_id),
            ..dispute_evidence
        },
        api::EvidenceType::UncategorizedFile => DisputeEvidence {
            uncategorized_file: Some(file_id),
            ..dispute_evidence
        },
    }
}
