use error_stack::{IntoReport, ResultExt};
use router_env::{instrument, tracing};

use super::errors::{self, RouterResponse, StorageErrorExt};
use crate::{
    routes::AppState,
    services,
    types::{api::disputes, storage, transformers::ForeignTryFrom},
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
