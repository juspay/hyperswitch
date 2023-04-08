use disputes::AcceptDisputeRequestData;
use error_stack::{IntoReport, ResultExt};
use router_env::{instrument, tracing};

use super::errors::{self, RouterResponse, StorageErrorExt};
use crate::{
    routes::AppState,
    services,
    types::{api::{disputes, self}, storage, transformers::{ForeignTryFrom, ForeignInto}},
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
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::DisputeNotFound {
                dispute_id: req.dispute_id,
            })
        })?;
    let dispute_response = api_models::disputes::DisputeResponse::foreign_try_from(dispute)
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
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
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::InternalServerError)
        })?;
    let mut disputes_list: Vec<api_models::disputes::DisputeResponse> = vec![];
    for dispute in disputes {
        let dispute_response = api_models::disputes::DisputeResponse::foreign_try_from(dispute)
            .into_report()
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        disputes_list.push(dispute_response);
    }
    Ok(services::ApplicationResponse::Json(disputes_list))
}

#[instrument(skip(state))]
pub async fn accept_dispute(
    state: &AppState,
    merchant_account: storage::MerchantAccount,
    req: disputes::DisputeId,
) -> RouterResponse<api_models::disputes::AcceptDisputeResponse> {
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
            api_models::disputes::AcceptDisputeResponse,
        > = connector_data.connector.get_connector_integration();
    let router_data = super::utils::construct_accept_dispute_router_data(
        state,
        &payment_intent,
        &payment_attempt,
        &merchant_account,
        &dispute,
    ).await?;
    let response = services::execute_connector_processing_step(
            state,
            connector_integration,
            &router_data,
            super::payments::CallConnectorAction::Trigger,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let accept_dispute_response = response.response
        .map_err(|_error| {
            errors::ApiErrorResponse::InternalServerError
        })?;
    let update_dispute = storage_models::dispute::DisputeUpdate::StatusUpdate {
        dispute_stage: accept_dispute_response.dispute_stage.clone().foreign_into(),
        dispute_status: accept_dispute_response.dispute_status.clone().foreign_into(),
        connector_status: None,
    };
    db.update_dispute(dispute, update_dispute)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    Ok(services::ApplicationResponse::Json(accept_dispute_response))
}
