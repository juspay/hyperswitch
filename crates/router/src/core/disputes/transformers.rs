use api_models::disputes::EvidenceType;
use common_utils::errors::CustomResult;
use error_stack::ResultExt;

use crate::{
    core::{errors, files::helpers::retrieve_file_and_provider_file_id_from_file_id},
    routes::SessionState,
    types::{
        api::{self, DisputeEvidence},
        domain,
        transformers::ForeignFrom,
        SubmitEvidenceRequestData,
    },
};

pub async fn get_evidence_request_data(
    state: &SessionState,
    platform: &domain::Platform,
    evidence_request: api_models::disputes::SubmitEvidenceRequest,
    dispute: &diesel_models::dispute::Dispute,
) -> CustomResult<SubmitEvidenceRequestData, errors::ApiErrorResponse> {
    let cancellation_policy_file_info = retrieve_file_and_provider_file_id_from_file_id(
        state,
        evidence_request.cancellation_policy,
        None,
        platform,
        api::FileDataRequired::NotRequired,
    )
    .await?;
    let customer_communication_file_info = retrieve_file_and_provider_file_id_from_file_id(
        state,
        evidence_request.customer_communication,
        None,
        platform,
        api::FileDataRequired::NotRequired,
    )
    .await?;
    let customer_sifnature_file_info = retrieve_file_and_provider_file_id_from_file_id(
        state,
        evidence_request.customer_signature,
        None,
        platform,
        api::FileDataRequired::NotRequired,
    )
    .await?;
    let receipt_file_info = retrieve_file_and_provider_file_id_from_file_id(
        state,
        evidence_request.receipt,
        None,
        platform,
        api::FileDataRequired::NotRequired,
    )
    .await?;
    let refund_policy_file_info = retrieve_file_and_provider_file_id_from_file_id(
        state,
        evidence_request.refund_policy,
        None,
        platform,
        api::FileDataRequired::NotRequired,
    )
    .await?;
    let service_documentation_file_info = retrieve_file_and_provider_file_id_from_file_id(
        state,
        evidence_request.service_documentation,
        None,
        platform,
        api::FileDataRequired::NotRequired,
    )
    .await?;
    let shipping_documentation_file_info = retrieve_file_and_provider_file_id_from_file_id(
        state,
        evidence_request.shipping_documentation,
        None,
        platform,
        api::FileDataRequired::NotRequired,
    )
    .await?;
    let invoice_showing_distinct_transactions_file_info =
        retrieve_file_and_provider_file_id_from_file_id(
            state,
            evidence_request.invoice_showing_distinct_transactions,
            None,
            platform,
            api::FileDataRequired::NotRequired,
        )
        .await?;
    let recurring_transaction_agreement_file_info =
        retrieve_file_and_provider_file_id_from_file_id(
            state,
            evidence_request.recurring_transaction_agreement,
            None,
            platform,
            api::FileDataRequired::NotRequired,
        )
        .await?;
    let uncategorized_file_info = retrieve_file_and_provider_file_id_from_file_id(
        state,
        evidence_request.uncategorized_file,
        None,
        platform,
        api::FileDataRequired::NotRequired,
    )
    .await?;
    Ok(SubmitEvidenceRequestData {
        dispute_id: dispute.dispute_id.clone(),
        dispute_status: dispute.dispute_status,
        connector_dispute_id: dispute.connector_dispute_id.clone(),
        access_activity_log: evidence_request.access_activity_log,
        billing_address: evidence_request.billing_address,
        cancellation_policy: cancellation_policy_file_info.file_data,
        cancellation_policy_provider_file_id: cancellation_policy_file_info.provider_file_id,
        cancellation_policy_disclosure: evidence_request.cancellation_policy_disclosure,
        cancellation_rebuttal: evidence_request.cancellation_rebuttal,
        customer_communication: customer_communication_file_info.file_data,
        customer_communication_provider_file_id: customer_communication_file_info.provider_file_id,
        customer_email_address: evidence_request.customer_email_address,
        customer_name: evidence_request.customer_name,
        customer_purchase_ip: evidence_request.customer_purchase_ip,
        customer_signature: customer_sifnature_file_info.file_data,
        customer_signature_provider_file_id: customer_sifnature_file_info.provider_file_id,
        product_description: evidence_request.product_description,
        receipt: receipt_file_info.file_data,
        receipt_provider_file_id: receipt_file_info.provider_file_id,
        refund_policy: refund_policy_file_info.file_data,
        refund_policy_provider_file_id: refund_policy_file_info.provider_file_id,
        refund_policy_disclosure: evidence_request.refund_policy_disclosure,
        refund_refusal_explanation: evidence_request.refund_refusal_explanation,
        service_date: evidence_request.service_date,
        service_documentation: service_documentation_file_info.file_data,
        service_documentation_provider_file_id: service_documentation_file_info.provider_file_id,
        shipping_address: evidence_request.shipping_address,
        shipping_carrier: evidence_request.shipping_carrier,
        shipping_date: evidence_request.shipping_date,
        shipping_documentation: shipping_documentation_file_info.file_data,
        shipping_documentation_provider_file_id: shipping_documentation_file_info.provider_file_id,
        shipping_tracking_number: evidence_request.shipping_tracking_number,
        invoice_showing_distinct_transactions: invoice_showing_distinct_transactions_file_info
            .file_data,
        invoice_showing_distinct_transactions_provider_file_id:
            invoice_showing_distinct_transactions_file_info.provider_file_id,
        recurring_transaction_agreement: recurring_transaction_agreement_file_info.file_data,
        recurring_transaction_agreement_provider_file_id: recurring_transaction_agreement_file_info
            .provider_file_id,
        uncategorized_file: uncategorized_file_info.file_data,
        uncategorized_file_provider_file_id: uncategorized_file_info.provider_file_id,
        uncategorized_text: evidence_request.uncategorized_text,
        cancellation_policy_file_type: cancellation_policy_file_info.file_type,
        customer_communication_file_type: customer_communication_file_info.file_type,
        customer_signature_file_type: customer_sifnature_file_info.file_type,
        receipt_file_type: receipt_file_info.file_type,
        refund_policy_file_type: refund_policy_file_info.file_type,
        service_documentation_file_type: service_documentation_file_info.file_type,
        shipping_documentation_file_type: shipping_documentation_file_info.file_type,
        invoice_showing_distinct_transactions_file_type:
            invoice_showing_distinct_transactions_file_info.file_type,
        recurring_transaction_agreement_file_type: recurring_transaction_agreement_file_info
            .file_type,
        uncategorized_file_type: uncategorized_file_info.file_type,
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

pub async fn get_dispute_evidence_block(
    state: &SessionState,
    platform: &domain::Platform,
    evidence_type: EvidenceType,
    file_id: String,
) -> CustomResult<api_models::disputes::DisputeEvidenceBlock, errors::ApiErrorResponse> {
    let file_metadata = state
        .store
        .find_file_metadata_by_merchant_id_file_id(
            platform.get_processor().get_account().get_id(),
            &file_id,
        )
        .await
        .change_context(errors::ApiErrorResponse::FileNotFound)
        .attach_printable("Unable to retrieve file_metadata")?;
    let file_metadata_response =
        api_models::files::FileMetadataResponse::foreign_from(file_metadata);
    Ok(api_models::disputes::DisputeEvidenceBlock {
        evidence_type,
        file_metadata_response,
    })
}

pub fn delete_evidence_file(
    dispute_evidence: DisputeEvidence,
    evidence_type: EvidenceType,
) -> DisputeEvidence {
    match evidence_type {
        EvidenceType::CancellationPolicy => DisputeEvidence {
            cancellation_policy: None,
            ..dispute_evidence
        },
        EvidenceType::CustomerCommunication => DisputeEvidence {
            customer_communication: None,
            ..dispute_evidence
        },
        EvidenceType::CustomerSignature => DisputeEvidence {
            customer_signature: None,
            ..dispute_evidence
        },
        EvidenceType::Receipt => DisputeEvidence {
            receipt: None,
            ..dispute_evidence
        },
        EvidenceType::RefundPolicy => DisputeEvidence {
            refund_policy: None,
            ..dispute_evidence
        },
        EvidenceType::ServiceDocumentation => DisputeEvidence {
            service_documentation: None,
            ..dispute_evidence
        },
        EvidenceType::ShippingDocumentation => DisputeEvidence {
            shipping_documentation: None,
            ..dispute_evidence
        },
        EvidenceType::InvoiceShowingDistinctTransactions => DisputeEvidence {
            invoice_showing_distinct_transactions: None,
            ..dispute_evidence
        },
        EvidenceType::RecurringTransactionAgreement => DisputeEvidence {
            recurring_transaction_agreement: None,
            ..dispute_evidence
        },
        EvidenceType::UncategorizedFile => DisputeEvidence {
            uncategorized_file: None,
            ..dispute_evidence
        },
    }
}

pub async fn get_dispute_evidence_vec(
    state: &SessionState,
    platform: domain::Platform,
    dispute_evidence: DisputeEvidence,
) -> CustomResult<Vec<api_models::disputes::DisputeEvidenceBlock>, errors::ApiErrorResponse> {
    let mut dispute_evidence_blocks: Vec<api_models::disputes::DisputeEvidenceBlock> = vec![];
    if let Some(cancellation_policy_block) = dispute_evidence.cancellation_policy {
        dispute_evidence_blocks.push(
            get_dispute_evidence_block(
                state,
                &platform,
                EvidenceType::CancellationPolicy,
                cancellation_policy_block,
            )
            .await?,
        )
    }
    if let Some(customer_communication_block) = dispute_evidence.customer_communication {
        dispute_evidence_blocks.push(
            get_dispute_evidence_block(
                state,
                &platform,
                EvidenceType::CustomerCommunication,
                customer_communication_block,
            )
            .await?,
        )
    }
    if let Some(customer_signature_block) = dispute_evidence.customer_signature {
        dispute_evidence_blocks.push(
            get_dispute_evidence_block(
                state,
                &platform,
                EvidenceType::CustomerSignature,
                customer_signature_block,
            )
            .await?,
        )
    }
    if let Some(receipt_block) = dispute_evidence.receipt {
        dispute_evidence_blocks.push(
            get_dispute_evidence_block(state, &platform, EvidenceType::Receipt, receipt_block)
                .await?,
        )
    }
    if let Some(refund_policy_block) = dispute_evidence.refund_policy {
        dispute_evidence_blocks.push(
            get_dispute_evidence_block(
                state,
                &platform,
                EvidenceType::RefundPolicy,
                refund_policy_block,
            )
            .await?,
        )
    }
    if let Some(service_documentation_block) = dispute_evidence.service_documentation {
        dispute_evidence_blocks.push(
            get_dispute_evidence_block(
                state,
                &platform,
                EvidenceType::ServiceDocumentation,
                service_documentation_block,
            )
            .await?,
        )
    }
    if let Some(shipping_documentation_block) = dispute_evidence.shipping_documentation {
        dispute_evidence_blocks.push(
            get_dispute_evidence_block(
                state,
                &platform,
                EvidenceType::ShippingDocumentation,
                shipping_documentation_block,
            )
            .await?,
        )
    }
    if let Some(invoice_showing_distinct_transactions_block) =
        dispute_evidence.invoice_showing_distinct_transactions
    {
        dispute_evidence_blocks.push(
            get_dispute_evidence_block(
                state,
                &platform,
                EvidenceType::InvoiceShowingDistinctTransactions,
                invoice_showing_distinct_transactions_block,
            )
            .await?,
        )
    }
    if let Some(recurring_transaction_agreement_block) =
        dispute_evidence.recurring_transaction_agreement
    {
        dispute_evidence_blocks.push(
            get_dispute_evidence_block(
                state,
                &platform,
                EvidenceType::RecurringTransactionAgreement,
                recurring_transaction_agreement_block,
            )
            .await?,
        )
    }
    if let Some(uncategorized_file_block) = dispute_evidence.uncategorized_file {
        dispute_evidence_blocks.push(
            get_dispute_evidence_block(
                state,
                &platform,
                EvidenceType::UncategorizedFile,
                uncategorized_file_block,
            )
            .await?,
        )
    }
    Ok(dispute_evidence_blocks)
}
