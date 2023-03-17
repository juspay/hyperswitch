use error_stack::{report, ResultExt};
use router_env::{instrument, logger, tracing};
use storage_models::enums as storage_enums;

use super::payments::helpers;
use crate::{
    core::errors::{self, RouterResponse, StorageErrorExt},
    db::StorageInterface,
    routes::AppState,
    services,
    types::{
        self,
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
    Ok(services::ApplicationResponse::Json(
        mandates::MandateResponse::from_db_mandate(state, mandate, &merchant_account).await?,
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

    Ok(services::ApplicationResponse::Json(
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
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!(
                "Failed while finding mandate: merchant_id: {}, customer_id: {}",
                merchant_account.merchant_id, req.customer_id
            )
        })?;

    if mandates.is_empty() {
        Err(report!(errors::ApiErrorResponse::MandateNotFound).attach_printable("No Mandate found"))
    } else {
        let mut response_vec = Vec::with_capacity(mandates.len());
        for mandate in mandates {
            response_vec.push(
                mandates::MandateResponse::from_db_mandate(state, mandate, &merchant_account)
                    .await?,
            );
        }
        Ok(services::ApplicationResponse::Json(response_vec))
    }
}

pub async fn mandate_procedure<F, FData>(
    state: &AppState,
    mut resp: types::RouterData<F, FData, types::PaymentsResponseData>,
    maybe_customer: &Option<storage::Customer>,
    merchant_account: &storage::MerchantAccount,
) -> errors::RouterResult<types::RouterData<F, FData, types::PaymentsResponseData>>
where
    FData: MandateBehaviour,
{
    match resp.request.get_mandate_id() {
        Some(mandate_id) => {
            let mandate_id = &mandate_id.mandate_id;
            let mandate = state
                .store
                .find_mandate_by_merchant_id_mandate_id(resp.merchant_id.as_ref(), mandate_id)
                .await
                .change_context(errors::ApiErrorResponse::MandateNotFound)?;
            let mandate = match mandate.mandate_type {
                storage_enums::MandateType::SingleUse => state
                    .store
                    .update_mandate_by_merchant_id_mandate_id(
                        &resp.merchant_id,
                        mandate_id,
                        storage::MandateUpdate::StatusUpdate {
                            mandate_status: storage_enums::MandateStatus::Revoked,
                        },
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::MandateNotFound),
                storage_enums::MandateType::MultiUse => state
                    .store
                    .update_mandate_by_merchant_id_mandate_id(
                        &resp.merchant_id,
                        mandate_id,
                        storage::MandateUpdate::CaptureAmountUpdate {
                            amount_captured: Some(
                                mandate.amount_captured.unwrap_or(0) + resp.request.get_amount(),
                            ),
                        },
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::MandateNotFound),
            }?;

            resp.payment_method_id = Some(mandate.payment_method_id);
        }
        None => {
            if resp.request.get_setup_mandate_details().is_some() {
                let payment_method_id = helpers::call_payment_method(
                    state,
                    merchant_account,
                    Some(&resp.request.get_payment_method_data()),
                    Some(resp.payment_method),
                    maybe_customer,
                )
                .await?
                .payment_method_id;

                resp.payment_method_id = Some(payment_method_id.clone());
                let mandate_reference = match resp.response.as_ref().ok() {
                    Some(types::PaymentsResponseData::TransactionResponse {
                        mandate_reference,
                        ..
                    }) => mandate_reference.clone(),
                    _ => None,
                };

                if let Some(new_mandate_data) = helpers::generate_mandate(
                    resp.merchant_id.clone(),
                    resp.connector.clone(),
                    resp.request.get_setup_mandate_details().map(Clone::clone),
                    maybe_customer,
                    payment_method_id,
                    mandate_reference,
                ) {
                    logger::debug!("{:?}", new_mandate_data);
                    resp.request
                        .set_mandate_id(api_models::payments::MandateIds {
                            mandate_id: new_mandate_data.mandate_id.clone(),
                            connector_mandate_id: new_mandate_data.connector_mandate_id.clone(),
                        });
                    state
                        .store
                        .insert_mandate(new_mandate_data)
                        .await
                        .map_err(|err| {
                            err.to_duplicate_response(
                                errors::ApiErrorResponse::DuplicateRefundRequest,
                            )
                        })?;
                };
            } else if resp.request.get_setup_future_usage().is_some() {
                helpers::call_payment_method(
                    state,
                    merchant_account,
                    Some(&resp.request.get_payment_method_data()),
                    Some(resp.payment_method),
                    maybe_customer,
                )
                .await?;
            }
        }
    }

    Ok(resp)
}

pub trait MandateBehaviour {
    fn get_amount(&self) -> i64;
    fn get_setup_future_usage(&self) -> Option<storage_models::enums::FutureUsage>;
    fn get_mandate_id(&self) -> Option<&api_models::payments::MandateIds>;
    fn set_mandate_id(&mut self, new_mandate_id: api_models::payments::MandateIds);
    fn get_payment_method_data(&self) -> api_models::payments::PaymentMethodData;
    fn get_setup_mandate_details(&self) -> Option<&api_models::payments::MandateData>;
}
