use common_utils::errors::CustomResult;

use crate::{
    core::{errors, files::helpers::retrieve_file_and_provider_file_id_from_file_id},
    routes::AppState,
    types::SubmitEvidenceRequestData,
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
        )
        .await?;
    let (customer_communication, customer_communication_provider_file_id) =
        retrieve_file_and_provider_file_id_from_file_id(
            state,
            evidence_request.customer_communication,
            merchant_account,
        )
        .await?;
    let (customer_signature, customer_signature_provider_file_id) =
        retrieve_file_and_provider_file_id_from_file_id(
            state,
            evidence_request.customer_signature,
            merchant_account,
        )
        .await?;
    let (receipt, receipt_provider_file_id) = retrieve_file_and_provider_file_id_from_file_id(
        state,
        evidence_request.receipt,
        merchant_account,
    )
    .await?;
    let (refund_policy, refund_policy_provider_file_id) =
        retrieve_file_and_provider_file_id_from_file_id(
            state,
            evidence_request.refund_policy,
            merchant_account,
        )
        .await?;
    let (service_documentation, service_documentation_provider_file_id) =
        retrieve_file_and_provider_file_id_from_file_id(
            state,
            evidence_request.service_documentation,
            merchant_account,
        )
        .await?;
    let (shipping_documentation, shipping_documentation_provider_file_id) =
        retrieve_file_and_provider_file_id_from_file_id(
            state,
            evidence_request.shipping_documentation,
            merchant_account,
        )
        .await?;
    let (uncategorized_file, uncategorized_file_provider_file_id) =
        retrieve_file_and_provider_file_id_from_file_id(
            state,
            evidence_request.uncategorized_file,
            merchant_account,
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
        uncategorized_file,
        uncategorized_file_provider_file_id,
        uncategorized_text: evidence_request.uncategorized_text,
    })
}
