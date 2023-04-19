use error_stack::ResultExt;
use router_env::{instrument, tracing};

use super::{
    errors::{self, RouterResponse, StorageErrorExt},
    metrics,
};
use crate::{
    routes::AppState,
    services,
    types::{
        api::{self, disputes},
        storage::{self, enums as storage_enums},
        transformers::{ForeignFrom, ForeignInto},
        AcceptDisputeRequestData, AcceptDisputeResponse,
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
        .to_not_found_response(errors::ApiErrorResponse::InternalServerError)?;
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
) -> RouterResponse<api_models::disputes::DisputeResponse> {
    let db = &state.store;
    let dispute = state
        .store
        .find_dispute_by_merchant_id_dispute_id(&merchant_account.merchant_id, &req.dispute_id)
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::DisputeNotFound {
                dispute_id: req.dispute_id,
            })
        })?;
    let dispute_id = dispute.dispute_id.clone();
    if !(dispute.dispute_stage == storage_enums::DisputeStage::Dispute
        && dispute.dispute_status == storage_enums::DisputeStatus::DisputeOpened)
    {
        metrics::ACCEPT_DISPUTE_STATUS_VALIDATION_FAILURE_METRIC.add(&metrics::CONTEXT, 1, &[]);
        Err(errors::ApiErrorResponse::DisputeStatusValidationFailed {
            reason: format!(
                "Dispute is in {} stage and has {} status",
                dispute.dispute_stage, dispute.dispute_status
            ),
        })?
    }
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
    let router_data = super::utils::construct_accept_dispute_router_data(
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
        super::payments::CallConnectorAction::Trigger,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;
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
