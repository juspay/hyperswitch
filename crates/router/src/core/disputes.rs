use router_env::{instrument, tracing};

use super::errors::{self, RouterResponse, StorageErrorExt};
use crate::{
    routes::AppState,
    services,
    types::{api::disputes, storage, transformers::ForeignFrom},
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
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::InternalServerError)
        })?;
    let disputes_list = disputes
        .into_iter()
        .map(api_models::disputes::DisputeResponse::foreign_from)
        .collect();
    Ok(services::ApplicationResponse::Json(disputes_list))
}
