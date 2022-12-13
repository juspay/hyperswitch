use error_stack::{report, ResultExt};
use router_env::{tracing, tracing::instrument};

use crate::{
    core::errors::{self, RouterResponse, StorageErrorExt},
    db::StorageInterface,
    routes::AppState,
    services,
    types::{
        api::{
            customers,
            mandates::{self, MandateResponseExt},
        },
        storage,
        transformers::ForeignInto,
    },
};

#[instrument(skip(state))]
pub async fn get_mandate(
    state: &AppState,
    merchant_account: storage::MerchantAccount,
    req: mandates::MandateId,
) -> RouterResponse<mandates::MandateResponse> {
    let mandate = state
        .store
        .find_mandate_by_merchant_id_mandate_id(&merchant_account.merchant_id, &req.mandate_id)
        .await
        .map_err(|error| error.to_not_found_response(errors::ApiErrorResponse::MandateNotFound))?;
    Ok(services::BachResponse::Json(
        mandates::MandateResponse::from_db_mandate(state, mandate).await?,
    ))
}

#[instrument(skip(db))]
pub async fn revoke_mandate(
    db: &dyn StorageInterface,
    merchant_account: storage::MerchantAccount,
    req: mandates::MandateId,
) -> RouterResponse<mandates::MandateRevokedResponse> {
    let mandate = db
        .update_mandate_by_merchant_id_mandate_id(
            &merchant_account.merchant_id,
            &req.mandate_id,
            storage::MandateUpdate::StatusUpdate {
                mandate_status: storage::enums::MandateStatus::Revoked,
            },
        )
        .await
        .map_err(|error| error.to_not_found_response(errors::ApiErrorResponse::MandateNotFound))?;

    Ok(services::BachResponse::Json(
        mandates::MandateRevokedResponse {
            mandate_id: mandate.mandate_id,
            status: mandate.mandate_status.foreign_into(),
        },
    ))
}

#[instrument(skip(state))]
pub async fn get_customer_mandates(
    state: &AppState,
    merchant_account: storage::MerchantAccount,
    req: customers::CustomerId,
) -> RouterResponse<Vec<mandates::MandateResponse>> {
    let mandates = state
        .store
        .find_mandate_by_merchant_id_customer_id(&merchant_account.merchant_id, &req.customer_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    if mandates.is_empty() {
        Err(report!(errors::ApiErrorResponse::MandateNotFound).attach_printable("No Mandate found"))
    } else {
        let mut response_vec = Vec::with_capacity(mandates.len());
        for mandate in mandates {
            response_vec.push(mandates::MandateResponse::from_db_mandate(state, mandate).await?);
        }
        Ok(services::BachResponse::Json(response_vec))
    }
}
