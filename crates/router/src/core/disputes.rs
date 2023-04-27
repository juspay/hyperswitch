use api_models::disputes as dispute_models;
use error_stack::ResultExt;
use router_env::{instrument, tracing};
pub mod transformers;

use super::{
    errors::{self, RouterResponse, StorageErrorExt},
    metrics,
};
use crate::{
    core::{payments, utils},
    routes::AppState,
    services,
    types::{
        api::{self, disputes},
        storage::{self, enums as storage_enums},
        transformers::{ForeignFrom, ForeignInto},
        AcceptDisputeRequestData, AcceptDisputeResponse, DefendDisputeRequestData,
        DefendDisputeResponse, SubmitEvidenceRequestData, SubmitEvidenceResponse,
    },
};

#[instrument(skip(state))]
pub async fn retrieve_dispute(
    state: &AppState,
    merchant_account: storage::MerchantAccount,
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
    state: &AppState,
    merchant_account: storage::MerchantAccount,
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
    state: &AppState,
    merchant_account: storage::MerchantAccount,
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
    )?;
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api::Accept,
        AcceptDisputeRequestData,
        AcceptDisputeResponse,
    > = connector_data.connector.get_connector_integration();
    let router_data = utils::construct_accept_dispute_router_data(
        state,
        &payment_intent,
        &payment_attempt,
        &merchant_account,
        &dispute,
    )
    .await?;
    let response = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
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
    let update_dispute = storage_models::dispute::DisputeUpdate::StatusUpdate {
        dispute_status: accept_dispute_response
            .dispute_status
            .clone()
            .foreign_into(),
        connector_status: accept_dispute_response.connector_status.clone(),
    };
    let updated_dispute = db
        .update_dispute(dispute.clone(), update_dispute)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!("Unable to update dispute with dispute_id: {}", dispute_id)
        })?;
    let dispute_response = api_models::disputes::DisputeResponse::foreign_from(updated_dispute);
    Ok(services::ApplicationResponse::Json(dispute_response))
}

#[instrument(skip(state))]
pub async fn submit_evidence(
    state: &AppState,
    merchant_account: storage::MerchantAccount,
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
    let submit_evidence_request_data =
        transformers::get_evidence_request_data(state, &merchant_account, req, &dispute).await?;
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
    )?;
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api::Evidence,
        SubmitEvidenceRequestData,
        SubmitEvidenceResponse,
    > = connector_data.connector.get_connector_integration();
    let router_data = utils::construct_submit_evidence_router_data(
        state,
        &payment_intent,
        &payment_attempt,
        &merchant_account,
        &dispute,
        submit_evidence_request_data,
    )
    .await?;
    let response = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
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
            let defend_dispute_router_data = utils::construct_defend_dispute_router_data(
                state,
                &payment_intent,
                &payment_attempt,
                &merchant_account,
                &dispute,
            )
            .await?;
            let defend_response = services::execute_connector_processing_step(
                state,
                connector_integration_defend_dispute,
                &defend_dispute_router_data,
                payments::CallConnectorAction::Trigger,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
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
    let update_dispute = storage_models::dispute::DisputeUpdate::StatusUpdate {
        dispute_status: dispute_status.foreign_into(),
        connector_status,
    };
    let updated_dispute = db
        .update_dispute(dispute.clone(), update_dispute)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!("Unable to update dispute with dispute_id: {}", dispute_id)
        })?;
    let dispute_response = api_models::disputes::DisputeResponse::foreign_from(updated_dispute);
    Ok(services::ApplicationResponse::Json(dispute_response))
}
