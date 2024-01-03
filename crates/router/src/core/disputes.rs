use api_models::{disputes as dispute_models, files as files_api_models};
use common_utils::ext_traits::ValueExt;
use error_stack::{IntoReport, ResultExt};
use router_env::{instrument, tracing};
pub mod transformers;

use super::{
    errors::{self, ConnectorErrorExt, RouterResponse, StorageErrorExt},
    metrics,
};
use crate::{
    core::{files, payments, utils as core_utils},
    routes::AppState,
    services,
    types::{
        api::{self, disputes},
        domain,
        storage::enums as storage_enums,
        transformers::ForeignFrom,
        AcceptDisputeRequestData, AcceptDisputeResponse, DefendDisputeRequestData,
        DefendDisputeResponse, SubmitEvidenceRequestData, SubmitEvidenceResponse,
    },
    utils,
};

#[instrument(skip(state))]
pub async fn retrieve_dispute(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    req: disputes::DisputeId,
) -> RouterResponse<api_models::disputes::DisputeResponse> {
    let dispute = state
        .store
        .find_dispute_by_merchant_id_dispute_id(&merchant_account.merchant_id, &req.dispute_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::DisputeNotFound {
            dispute_id: req.dispute_id,
        })?;
    let dispute_response = api_models::disputes::DisputeResponse::foreign_from(dispute);
    Ok(services::ApplicationResponse::Json(dispute_response))
}

#[instrument(skip(state))]
pub async fn retrieve_disputes_list(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    constraints: api_models::disputes::DisputeListConstraints,
) -> RouterResponse<Vec<api_models::disputes::DisputeResponse>> {
    let disputes = state
        .store
        .find_disputes_by_merchant_id(&merchant_account.merchant_id, constraints)
        .await
        .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to retrieve disputes")?;
    let disputes_list = disputes
        .into_iter()
        .map(api_models::disputes::DisputeResponse::foreign_from)
        .collect();
    Ok(services::ApplicationResponse::Json(disputes_list))
}

#[instrument(skip(state))]
pub async fn accept_dispute(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: disputes::DisputeId,
) -> RouterResponse<dispute_models::DisputeResponse> {
    let db = &state.store;
    let dispute = state
        .store
        .find_dispute_by_merchant_id_dispute_id(&merchant_account.merchant_id, &req.dispute_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::DisputeNotFound {
            dispute_id: req.dispute_id,
        })?;
    let dispute_id = dispute.dispute_id.clone();
    common_utils::fp_utils::when(
        !(dispute.dispute_stage == storage_enums::DisputeStage::Dispute
            && dispute.dispute_status == storage_enums::DisputeStatus::DisputeOpened),
        || {
            metrics::ACCEPT_DISPUTE_STATUS_VALIDATION_FAILURE_METRIC.add(&metrics::CONTEXT, 1, &[]);
            Err(errors::ApiErrorResponse::DisputeStatusValidationFailed {
            reason: format!(
                "This dispute cannot be accepted because the dispute is in {} stage and has {} status",
                dispute.dispute_stage, dispute.dispute_status
            ),
        })
        },
    )?;
    let payment_intent = db
        .find_payment_intent_by_payment_id_merchant_id(
            &dispute.payment_id,
            &merchant_account.merchant_id,
            merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::PaymentNotFound)?;
    let payment_attempt = db
        .find_payment_attempt_by_attempt_id_merchant_id(
            &dispute.attempt_id,
            &merchant_account.merchant_id,
            merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::PaymentNotFound)?;
    let connector_data = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &dispute.connector,
        api::GetToken::Connector,
        dispute.merchant_connector_id.clone(),
    )?;
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api::Accept,
        AcceptDisputeRequestData,
        AcceptDisputeResponse,
    > = connector_data.connector.get_connector_integration();
    let router_data = core_utils::construct_accept_dispute_router_data(
        &state,
        &payment_intent,
        &payment_attempt,
        &merchant_account,
        &key_store,
        &dispute,
    )
    .await?;
    let response = services::execute_connector_processing_step(
        &state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await
    .to_dispute_failed_response()
    .attach_printable("Failed while calling accept dispute connector api")?;
    let accept_dispute_response =
        response
            .response
            .map_err(|err| errors::ApiErrorResponse::ExternalConnectorError {
                code: err.code,
                message: err.message,
                connector: dispute.connector.clone(),
                status_code: err.status_code,
                reason: err.reason,
            })?;
    let update_dispute = diesel_models::dispute::DisputeUpdate::StatusUpdate {
        dispute_status: accept_dispute_response.dispute_status,
        connector_status: accept_dispute_response.connector_status.clone(),
    };
    let updated_dispute = db
        .update_dispute(dispute.clone(), update_dispute)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!("Unable to update dispute with dispute_id: {dispute_id}")
        })?;
    let dispute_response = api_models::disputes::DisputeResponse::foreign_from(updated_dispute);
    Ok(services::ApplicationResponse::Json(dispute_response))
}

#[instrument(skip(state))]
pub async fn submit_evidence(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: dispute_models::SubmitEvidenceRequest,
) -> RouterResponse<dispute_models::DisputeResponse> {
    let db = &state.store;
    let dispute = state
        .store
        .find_dispute_by_merchant_id_dispute_id(&merchant_account.merchant_id, &req.dispute_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::DisputeNotFound {
            dispute_id: req.dispute_id.clone(),
        })?;
    let dispute_id = dispute.dispute_id.clone();
    common_utils::fp_utils::when(
        !(dispute.dispute_stage == storage_enums::DisputeStage::Dispute
            && dispute.dispute_status == storage_enums::DisputeStatus::DisputeOpened),
        || {
            metrics::EVIDENCE_SUBMISSION_DISPUTE_STATUS_VALIDATION_FAILURE_METRIC.add(
                &metrics::CONTEXT,
                1,
                &[],
            );
            Err(errors::ApiErrorResponse::DisputeStatusValidationFailed {
                reason: format!(
                "Evidence cannot be submitted because the dispute is in {} stage and has {} status",
                dispute.dispute_stage, dispute.dispute_status
            ),
            })
        },
    )?;
    let submit_evidence_request_data = transformers::get_evidence_request_data(
        &state,
        &merchant_account,
        &key_store,
        req,
        &dispute,
    )
    .await?;
    let payment_intent = db
        .find_payment_intent_by_payment_id_merchant_id(
            &dispute.payment_id,
            &merchant_account.merchant_id,
            merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::PaymentNotFound)?;
    let payment_attempt = db
        .find_payment_attempt_by_attempt_id_merchant_id(
            &dispute.attempt_id,
            &merchant_account.merchant_id,
            merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::PaymentNotFound)?;
    let connector_data = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &dispute.connector,
        api::GetToken::Connector,
        dispute.merchant_connector_id.clone(),
    )?;

    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api::Evidence,
        SubmitEvidenceRequestData,
        SubmitEvidenceResponse,
    > = connector_data.connector.get_connector_integration();
    let router_data = core_utils::construct_submit_evidence_router_data(
        &state,
        &payment_intent,
        &payment_attempt,
        &merchant_account,
        &key_store,
        &dispute,
        submit_evidence_request_data,
    )
    .await?;
    let response = services::execute_connector_processing_step(
        &state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await
    .to_dispute_failed_response()
    .attach_printable("Failed while calling submit evidence connector api")?;
    let submit_evidence_response =
        response
            .response
            .map_err(|err| errors::ApiErrorResponse::ExternalConnectorError {
                code: err.code,
                message: err.message,
                connector: dispute.connector.clone(),
                status_code: err.status_code,
                reason: err.reason,
            })?;
    //Defend Dispute Optionally if connector expects to defend / submit evidence in a separate api call
    let (dispute_status, connector_status) =
        if connector_data.connector_name.requires_defend_dispute() {
            let connector_integration_defend_dispute: services::BoxedConnectorIntegration<
                '_,
                api::Defend,
                DefendDisputeRequestData,
                DefendDisputeResponse,
            > = connector_data.connector.get_connector_integration();
            let defend_dispute_router_data = core_utils::construct_defend_dispute_router_data(
                &state,
                &payment_intent,
                &payment_attempt,
                &merchant_account,
                &key_store,
                &dispute,
            )
            .await?;
            let defend_response = services::execute_connector_processing_step(
                &state,
                connector_integration_defend_dispute,
                &defend_dispute_router_data,
                payments::CallConnectorAction::Trigger,
                None,
            )
            .await
            .to_dispute_failed_response()
            .attach_printable("Failed while calling defend dispute connector api")?;
            let defend_dispute_response = defend_response.response.map_err(|err| {
                errors::ApiErrorResponse::ExternalConnectorError {
                    code: err.code,
                    message: err.message,
                    connector: dispute.connector.clone(),
                    status_code: err.status_code,
                    reason: err.reason,
                }
            })?;
            (
                defend_dispute_response.dispute_status,
                defend_dispute_response.connector_status,
            )
        } else {
            (
                submit_evidence_response.dispute_status,
                submit_evidence_response.connector_status,
            )
        };
    let update_dispute = diesel_models::dispute::DisputeUpdate::StatusUpdate {
        dispute_status,
        connector_status,
    };
    let updated_dispute = db
        .update_dispute(dispute.clone(), update_dispute)
        .await
        .to_not_found_response(errors::ApiErrorResponse::DisputeNotFound {
            dispute_id: dispute_id.to_owned(),
        })
        .attach_printable_lazy(|| {
            format!("Unable to update dispute with dispute_id: {dispute_id}")
        })?;
    let dispute_response = api_models::disputes::DisputeResponse::foreign_from(updated_dispute);
    Ok(services::ApplicationResponse::Json(dispute_response))
}

pub async fn attach_evidence(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    attach_evidence_request: api::AttachEvidenceRequest,
) -> RouterResponse<files_api_models::CreateFileResponse> {
    let db = &state.store;
    let dispute_id = attach_evidence_request
        .create_file_request
        .dispute_id
        .clone()
        .ok_or(errors::ApiErrorResponse::MissingDisputeId)?;
    let dispute = db
        .find_dispute_by_merchant_id_dispute_id(&merchant_account.merchant_id, &dispute_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::DisputeNotFound {
            dispute_id: dispute_id.clone(),
        })?;
    common_utils::fp_utils::when(
        !(dispute.dispute_stage == storage_enums::DisputeStage::Dispute
            && dispute.dispute_status == storage_enums::DisputeStatus::DisputeOpened),
        || {
            metrics::ATTACH_EVIDENCE_DISPUTE_STATUS_VALIDATION_FAILURE_METRIC.add(
                &metrics::CONTEXT,
                1,
                &[],
            );
            Err(errors::ApiErrorResponse::DisputeStatusValidationFailed {
                reason: format!(
                "Evidence cannot be attached because the dispute is in {} stage and has {} status",
                dispute.dispute_stage, dispute.dispute_status
            ),
            })
        },
    )?;
    let create_file_response = files::files_create_core(
        state.clone(),
        merchant_account,
        key_store,
        attach_evidence_request.create_file_request,
    )
    .await?;
    let file_id = match &create_file_response {
        services::ApplicationResponse::Json(res) => res.file_id.clone(),
        _ => Err(errors::ApiErrorResponse::InternalServerError)
            .into_report()
            .attach_printable("Unexpected response received from files create core")?,
    };
    let dispute_evidence: api::DisputeEvidence = dispute
        .evidence
        .clone()
        .parse_value("DisputeEvidence")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error while parsing dispute evidence record")?;
    let updated_dispute_evidence = transformers::update_dispute_evidence(
        dispute_evidence,
        attach_evidence_request.evidence_type,
        file_id,
    );
    let update_dispute = diesel_models::dispute::DisputeUpdate::EvidenceUpdate {
        evidence: utils::Encode::<api::DisputeEvidence>::encode_to_value(&updated_dispute_evidence)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error while encoding dispute evidence")?
            .into(),
    };
    db.update_dispute(dispute, update_dispute)
        .await
        .to_not_found_response(errors::ApiErrorResponse::DisputeNotFound {
            dispute_id: dispute_id.to_owned(),
        })
        .attach_printable_lazy(|| {
            format!("Unable to update dispute with dispute_id: {dispute_id}")
        })?;
    Ok(create_file_response)
}

#[instrument(skip(state))]
pub async fn retrieve_dispute_evidence(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    req: disputes::DisputeId,
) -> RouterResponse<Vec<api_models::disputes::DisputeEvidenceBlock>> {
    let dispute = state
        .store
        .find_dispute_by_merchant_id_dispute_id(&merchant_account.merchant_id, &req.dispute_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::DisputeNotFound {
            dispute_id: req.dispute_id,
        })?;
    let dispute_evidence: api::DisputeEvidence = dispute
        .evidence
        .clone()
        .parse_value("DisputeEvidence")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error while parsing dispute evidence record")?;
    let dispute_evidence_vec =
        transformers::get_dispute_evidence_vec(&state, merchant_account, dispute_evidence).await?;
    Ok(services::ApplicationResponse::Json(dispute_evidence_vec))
}
