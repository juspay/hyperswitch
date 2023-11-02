use api_models::payments;
use common_utils::{ext_traits::Encode, pii};
use diesel_models::enums as storage_enums;
use error_stack::{report, ResultExt};
use futures::future;
use router_env::{instrument, logger, tracing};

use super::payments::helpers;
use crate::{
    core::errors::{self, RouterResponse, StorageErrorExt},
    db::StorageInterface,
    routes::{metrics, AppState},
    services,
    types::{
        self,
        api::{
            customers,
            mandates::{self, MandateResponseExt},
        },
        domain, storage,
        transformers::ForeignTryFrom,
    },
    utils::OptionExt,
};

#[instrument(skip(state))]
pub async fn get_mandate(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    req: mandates::MandateId,
) -> RouterResponse<mandates::MandateResponse> {
    let mandate = state
        .store
        .as_ref()
        .find_mandate_by_merchant_id_mandate_id(&merchant_account.merchant_id, &req.mandate_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MandateNotFound)?;
    Ok(services::ApplicationResponse::Json(
        mandates::MandateResponse::from_db_mandate(&state, mandate).await?,
    ))
}

#[instrument(skip(state))]
pub async fn revoke_mandate(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    req: mandates::MandateId,
) -> RouterResponse<mandates::MandateRevokedResponse> {
    let db = state.store.as_ref();
    let mandate = db
        .update_mandate_by_merchant_id_mandate_id(
            &merchant_account.merchant_id,
            &req.mandate_id,
            storage::MandateUpdate::StatusUpdate {
                mandate_status: storage::enums::MandateStatus::Revoked,
            },
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MandateNotFound)?;

    Ok(services::ApplicationResponse::Json(
        mandates::MandateRevokedResponse {
            mandate_id: mandate.mandate_id,
            status: mandate.mandate_status,
        },
    ))
}

#[instrument(skip(db))]
pub async fn update_connector_mandate_id(
    db: &dyn StorageInterface,
    merchant_account: String,
    mandate_ids_opt: Option<api_models::payments::MandateIds>,
    resp: Result<types::PaymentsResponseData, types::ErrorResponse>,
) -> RouterResponse<mandates::MandateResponse> {
    let connector_mandate_id = Option::foreign_try_from(resp)?;
    //Ignore updation if the payment_attempt mandate_id or connector_mandate_id is not present
    if let Some((mandate_ids, connector_id)) = mandate_ids_opt.zip(connector_mandate_id) {
        let mandate_id = &mandate_ids.mandate_id;
        let mandate = db
            .find_mandate_by_merchant_id_mandate_id(&merchant_account, mandate_id)
            .await
            .change_context(errors::ApiErrorResponse::MandateNotFound)?;
        // only update the connector_mandate_id if existing is none
        if mandate.connector_mandate_id.is_none() {
            db.update_mandate_by_merchant_id_mandate_id(
                &merchant_account,
                mandate_id,
                storage::MandateUpdate::ConnectorReferenceUpdate {
                    connector_mandate_ids: Some(connector_id),
                },
            )
            .await
            .change_context(errors::ApiErrorResponse::MandateUpdateFailed)?;
        }
    }
    Ok(services::ApplicationResponse::StatusOk)
}

#[instrument(skip(state))]
pub async fn get_customer_mandates(
    state: AppState,
    merchant_account: domain::MerchantAccount,
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
            response_vec.push(mandates::MandateResponse::from_db_mandate(&state, mandate).await?);
        }
        Ok(services::ApplicationResponse::Json(response_vec))
    }
}

fn get_insensitive_payment_method_data_if_exists<F, FData>(
    router_data: &types::RouterData<F, FData, types::PaymentsResponseData>,
) -> Option<payments::PaymentMethodData>
where
    FData: MandateBehaviour,
{
    match &router_data.request.get_payment_method_data() {
        api_models::payments::PaymentMethodData::Card(_) => None,
        _ => Some(router_data.request.get_payment_method_data()),
    }
}

pub async fn mandate_procedure<F, FData>(
    state: &AppState,
    mut resp: types::RouterData<F, FData, types::PaymentsResponseData>,
    maybe_customer: &Option<domain::Customer>,
    pm_id: Option<String>,
    merchant_connector_id: Option<String>,
) -> errors::RouterResult<types::RouterData<F, FData, types::PaymentsResponseData>>
where
    FData: MandateBehaviour,
{
    match resp.response {
        Err(_) => {}
        Ok(_) => match resp.request.get_mandate_id() {
            Some(mandate_id) => {
                let mandate_id = &mandate_id.mandate_id;
                let mandate = state
                    .store
                    .find_mandate_by_merchant_id_mandate_id(resp.merchant_id.as_ref(), mandate_id)
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::MandateNotFound)?;
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
                        .change_context(errors::ApiErrorResponse::MandateUpdateFailed),
                    storage_enums::MandateType::MultiUse => state
                        .store
                        .update_mandate_by_merchant_id_mandate_id(
                            &resp.merchant_id,
                            mandate_id,
                            storage::MandateUpdate::CaptureAmountUpdate {
                                amount_captured: Some(
                                    mandate.amount_captured.unwrap_or(0)
                                        + resp.request.get_amount(),
                                ),
                            },
                        )
                        .await
                        .change_context(errors::ApiErrorResponse::MandateUpdateFailed),
                }?;
                metrics::SUBSEQUENT_MANDATE_PAYMENT.add(
                    &metrics::CONTEXT,
                    1,
                    &[metrics::request::add_attributes(
                        "connector",
                        mandate.connector,
                    )],
                );
                resp.payment_method_id = Some(mandate.payment_method_id);
            }
            None => {
                if resp.request.get_setup_mandate_details().is_some() {
                    resp.payment_method_id = pm_id.clone();
                    let (mandate_reference, network_txn_id) = match resp.response.as_ref().ok() {
                        Some(types::PaymentsResponseData::TransactionResponse {
                            mandate_reference,
                            network_txn_id,
                            ..
                        }) => (mandate_reference.clone(), network_txn_id.clone()),
                        _ => (None, None),
                    };

                    let mandate_ids = mandate_reference
                        .as_ref()
                        .map(|md| {
                            Encode::<types::MandateReference>::encode_to_value(&md)
                                .change_context(
                                    errors::ApiErrorResponse::MandateSerializationFailed,
                                )
                                .map(masking::Secret::new)
                        })
                        .transpose()?;

                    if let Some(new_mandate_data) = helpers::generate_mandate(
                        resp.merchant_id.clone(),
                        resp.payment_id.clone(),
                        resp.connector.clone(),
                        resp.request.get_setup_mandate_details().map(Clone::clone),
                        maybe_customer,
                        pm_id.get_required_value("payment_method_id")?,
                        mandate_ids,
                        network_txn_id,
                        get_insensitive_payment_method_data_if_exists(&resp),
                        mandate_reference,
                        merchant_connector_id,
                    )? {
                        let connector = new_mandate_data.connector.clone();
                        logger::debug!("{:?}", new_mandate_data);
                        resp.request
                        .set_mandate_id(Some(api_models::payments::MandateIds {
                            mandate_id: new_mandate_data.mandate_id.clone(),
                            mandate_reference_id: new_mandate_data
                                .connector_mandate_ids
                                .clone()
                            .map(|ids| {
                                Some(ids)
                                    .parse_value::<api_models::payments::ConnectorMandateReferenceId>(
                                        "ConnectorMandateId",
                                    )
                                    .change_context(errors::ApiErrorResponse::MandateDeserializationFailed)
                            })
                            .transpose()?
                            .map_or(
                                new_mandate_data.network_transaction_id.clone().map(|id| {
                                    api_models::payments::MandateReferenceId::NetworkMandateId(
                                        id,
                                    )
                                }),
                                |connector_id| Some(api_models::payments::MandateReferenceId::ConnectorMandateId(
                                    api_models::payments::ConnectorMandateReferenceId {
                                        connector_mandate_id: connector_id.connector_mandate_id,
                                        payment_method_id: connector_id.payment_method_id,
                                    }
                                )))
                        }));
                        state
                            .store
                            .insert_mandate(new_mandate_data)
                            .await
                            .to_duplicate_response(errors::ApiErrorResponse::DuplicateMandate)?;
                        metrics::MANDATE_COUNT.add(
                            &metrics::CONTEXT,
                            1,
                            &[metrics::request::add_attributes("connector", connector)],
                        );
                    };
                }
            }
        },
    }
    Ok(resp)
}

#[instrument(skip(state))]
pub async fn retrieve_mandates_list(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    constraints: api_models::mandates::MandateListConstraints,
) -> RouterResponse<Vec<api_models::mandates::MandateResponse>> {
    let mandates = state
        .store
        .as_ref()
        .find_mandates_by_merchant_id(&merchant_account.merchant_id, constraints)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to retrieve mandates")?;
    let mandates_list = future::try_join_all(
        mandates
            .into_iter()
            .map(|mandate| mandates::MandateResponse::from_db_mandate(&state, mandate)),
    )
    .await?;
    Ok(services::ApplicationResponse::Json(mandates_list))
}

impl ForeignTryFrom<Result<types::PaymentsResponseData, types::ErrorResponse>>
    for Option<pii::SecretSerdeValue>
{
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn foreign_try_from(
        resp: Result<types::PaymentsResponseData, types::ErrorResponse>,
    ) -> errors::RouterResult<Self> {
        let mandate_details = match resp {
            Ok(types::PaymentsResponseData::TransactionResponse {
                mandate_reference, ..
            }) => mandate_reference,
            _ => None,
        };

        mandate_details
            .map(|md| {
                Encode::<types::MandateReference>::encode_to_value(&md)
                    .change_context(errors::ApiErrorResponse::MandateNotFound)
                    .map(masking::Secret::new)
            })
            .transpose()
    }
}

pub trait MandateBehaviour {
    fn get_amount(&self) -> i64;
    fn get_setup_future_usage(&self) -> Option<diesel_models::enums::FutureUsage>;
    fn get_mandate_id(&self) -> Option<&api_models::payments::MandateIds>;
    fn set_mandate_id(&mut self, new_mandate_id: Option<api_models::payments::MandateIds>);
    fn get_payment_method_data(&self) -> api_models::payments::PaymentMethodData;
    fn get_setup_mandate_details(&self) -> Option<&data_models::mandates::MandateData>;
}
