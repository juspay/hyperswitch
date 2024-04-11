pub mod helpers;
pub mod utils;
use api_models::payments;
use common_utils::ext_traits::Encode;
use diesel_models::{enums as storage_enums, Mandate};
use error_stack::{report, ResultExt};
use futures::future;
use router_env::{instrument, logger, tracing};

use super::payments::helpers as payment_helper;
use crate::{
    core::{
        errors::{self, RouterResponse, StorageErrorExt},
        payments::CallConnectorAction,
    },
    db::StorageInterface,
    routes::{metrics, AppState},
    services,
    types::{
        self,
        api::{
            customers,
            mandates::{self, MandateResponseExt},
            ConnectorData, GetToken,
        },
        domain, storage,
        transformers::ForeignFrom,
    },
    utils::OptionExt,
};

#[instrument(skip(state))]
pub async fn get_mandate(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: mandates::MandateId,
) -> RouterResponse<mandates::MandateResponse> {
    let mandate = state
        .store
        .as_ref()
        .find_mandate_by_merchant_id_mandate_id(&merchant_account.merchant_id, &req.mandate_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MandateNotFound)?;
    Ok(services::ApplicationResponse::Json(
        mandates::MandateResponse::from_db_mandate(&state, key_store, mandate).await?,
    ))
}

#[instrument(skip(state))]
pub async fn revoke_mandate(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: mandates::MandateId,
) -> RouterResponse<mandates::MandateRevokedResponse> {
    let db = state.store.as_ref();
    let mandate = db
        .find_mandate_by_merchant_id_mandate_id(&merchant_account.merchant_id, &req.mandate_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MandateNotFound)?;
    match mandate.mandate_status {
        common_enums::MandateStatus::Active
        | common_enums::MandateStatus::Inactive
        | common_enums::MandateStatus::Pending => {
            let profile_id =
                helpers::get_profile_id_for_mandate(&state, &merchant_account, mandate.clone())
                    .await?;

            let merchant_connector_account = payment_helper::get_merchant_connector_account(
                &state,
                &merchant_account.merchant_id,
                None,
                &key_store,
                &profile_id,
                &mandate.connector.clone(),
                mandate.merchant_connector_id.as_ref(),
            )
            .await?;

            let connector_data = ConnectorData::get_connector_by_name(
                &state.conf.connectors,
                &mandate.connector,
                GetToken::Connector,
                mandate.merchant_connector_id.clone(),
            )?;
            let connector_integration: services::BoxedConnectorIntegration<
                '_,
                types::api::MandateRevoke,
                types::MandateRevokeRequestData,
                types::MandateRevokeResponseData,
            > = connector_data.connector.get_connector_integration();

            let router_data = utils::construct_mandate_revoke_router_data(
                merchant_connector_account,
                &merchant_account,
                mandate.clone(),
            )
            .await?;

            let response = services::execute_connector_processing_step(
                &state,
                connector_integration,
                &router_data,
                CallConnectorAction::Trigger,
                None,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

            match response.response {
                Ok(_) => {
                    let update_mandate = db
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
                            mandate_id: update_mandate.mandate_id,
                            status: update_mandate.mandate_status,
                            error_code: None,
                            error_message: None,
                        },
                    ))
                }

                Err(err) => Err(errors::ApiErrorResponse::ExternalConnectorError {
                    code: err.code,
                    message: err.message,
                    connector: mandate.connector,
                    status_code: err.status_code,
                    reason: err.reason,
                }
                .into()),
            }
        }
        common_enums::MandateStatus::Revoked => {
            Err(errors::ApiErrorResponse::MandateValidationFailed {
                reason: "Mandate has already been revoked".to_string(),
            }
            .into())
        }
    }
}

#[instrument(skip(db))]
pub async fn update_connector_mandate_id(
    db: &dyn StorageInterface,
    merchant_account: String,
    mandate_ids_opt: Option<String>,
    payment_method_id: Option<String>,
    resp: Result<types::PaymentsResponseData, types::ErrorResponse>,
) -> RouterResponse<mandates::MandateResponse> {
    let mandate_details = Option::foreign_from(resp);
    let connector_mandate_id = mandate_details
        .clone()
        .map(|md| {
            md.encode_to_value()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .map(masking::Secret::new)
        })
        .transpose()?;

    //Ignore updation if the payment_attempt mandate_id or connector_mandate_id is not present
    if let Some((mandate_id, connector_id)) = mandate_ids_opt.zip(connector_mandate_id) {
        let mandate = db
            .find_mandate_by_merchant_id_mandate_id(&merchant_account, &mandate_id)
            .await
            .change_context(errors::ApiErrorResponse::MandateNotFound)?;

        let update_mandate_details = match payment_method_id {
            Some(pmd_id) => storage::MandateUpdate::ConnectorMandateIdUpdate {
                connector_mandate_id: mandate_details
                    .and_then(|mandate_reference| mandate_reference.connector_mandate_id),
                connector_mandate_ids: Some(connector_id),
                payment_method_id: pmd_id,
                original_payment_id: None,
            },
            None => storage::MandateUpdate::ConnectorReferenceUpdate {
                connector_mandate_ids: Some(connector_id),
            },
        };

        // only update the connector_mandate_id if existing is none
        if mandate.connector_mandate_id.is_none() {
            db.update_mandate_by_merchant_id_mandate_id(
                &merchant_account,
                &mandate_id,
                update_mandate_details,
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
    key_store: domain::MerchantKeyStore,
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
                mandates::MandateResponse::from_db_mandate(&state, key_store.clone(), mandate)
                    .await?,
            );
        }
        Ok(services::ApplicationResponse::Json(response_vec))
    }
}

fn get_insensitive_payment_method_data_if_exists<F, FData>(
    router_data: &types::RouterData<F, FData, types::PaymentsResponseData>,
) -> Option<domain::PaymentMethodData>
where
    FData: MandateBehaviour,
{
    match &router_data.request.get_payment_method_data() {
        domain::PaymentMethodData::Card(_) => None,
        _ => Some(router_data.request.get_payment_method_data()),
    }
}
pub async fn update_mandate_procedure<F, FData>(
    state: &AppState,
    resp: types::RouterData<F, FData, types::PaymentsResponseData>,
    mandate: Mandate,
    merchant_id: &str,
    pm_id: Option<String>,
) -> errors::RouterResult<types::RouterData<F, FData, types::PaymentsResponseData>>
where
    FData: MandateBehaviour,
{
    let mandate_details = match &resp.response {
        Ok(types::PaymentsResponseData::TransactionResponse {
            mandate_reference, ..
        }) => mandate_reference,
        Ok(_) => Err(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unexpected response received")?,
        Err(_) => return Ok(resp),
    };

    let old_record = payments::UpdateHistory {
        connector_mandate_id: mandate.connector_mandate_id,
        payment_method_id: mandate.payment_method_id,
        original_payment_id: mandate.original_payment_id,
    };

    let mandate_ref = mandate
        .connector_mandate_ids
        .parse_value::<payments::ConnectorMandateReferenceId>("Connector Reference Id")
        .change_context(errors::ApiErrorResponse::MandateDeserializationFailed)?;

    let mut update_history = mandate_ref.update_history.unwrap_or_default();
    update_history.push(old_record);

    let updated_mandate_ref = payments::ConnectorMandateReferenceId {
        connector_mandate_id: mandate_details
            .as_ref()
            .and_then(|mandate_ref| mandate_ref.connector_mandate_id.clone()),
        payment_method_id: pm_id.clone(),
        update_history: Some(update_history),
    };

    let connector_mandate_ids = updated_mandate_ref
        .encode_to_value()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .map(masking::Secret::new)?;

    let _update_mandate_details = state
        .store
        .update_mandate_by_merchant_id_mandate_id(
            merchant_id,
            &mandate.mandate_id,
            diesel_models::MandateUpdate::ConnectorMandateIdUpdate {
                connector_mandate_id: mandate_details
                    .as_ref()
                    .and_then(|man_ref| man_ref.connector_mandate_id.clone()),
                connector_mandate_ids: Some(connector_mandate_ids),
                payment_method_id: pm_id
                    .unwrap_or("Error retrieving the payment_method_id".to_string()),
                original_payment_id: Some(resp.payment_id.clone()),
            },
        )
        .await
        .change_context(errors::ApiErrorResponse::MandateUpdateFailed)?;
    Ok(resp)
}

pub async fn mandate_procedure<F, FData>(
    state: &AppState,
    resp: &types::RouterData<F, FData, types::PaymentsResponseData>,
    customer_id: &Option<String>,
    pm_id: Option<String>,
    merchant_connector_id: Option<String>,
) -> errors::RouterResult<()>
where
    FData: MandateBehaviour,
{
    match resp.response {
        Err(_) => {}
        Ok(_) => match resp.request.get_mandate_id() {
            Some(mandate_id) => {
                if let Some(ref mandate_id) = mandate_id.mandate_id {
                    let mandate = state
                        .store
                        .find_mandate_by_merchant_id_mandate_id(
                            resp.merchant_id.as_ref(),
                            mandate_id,
                        )
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
                }
            }
            None => {
                if resp.request.get_setup_mandate_details().is_some() {
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
                            md.encode_to_value()
                                .change_context(
                                    errors::ApiErrorResponse::MandateSerializationFailed,
                                )
                                .map(masking::Secret::new)
                        })
                        .transpose()?;

                    if let Some(new_mandate_data) = payment_helper::generate_mandate(
                        resp.merchant_id.clone(),
                        resp.payment_id.clone(),
                        resp.connector.clone(),
                        resp.request.get_setup_mandate_details().cloned(),
                        customer_id,
                        pm_id.get_required_value("payment_method_id")?,
                        mandate_ids,
                        network_txn_id,
                        get_insensitive_payment_method_data_if_exists(resp),
                        mandate_reference,
                        merchant_connector_id,
                    )? {
                        let connector = new_mandate_data.connector.clone();
                        logger::debug!("{:?}", new_mandate_data);

                        // For GooglePay Mandates
                        // resp.request
                        //     .set_mandate_id(Some(api_models::payments::MandateIds {
                        //         mandate_id: Some(new_mandate_data.mandate_id.clone()),
                        //         mandate_reference_id: new_mandate_data
                        //             .connector_mandate_ids
                        //             .clone()
                        //         .map(|ids| {
                        //             Some(ids)
                        //                 .parse_value::<api_models::payments::ConnectorMandateReferenceId>(
                        //                     "ConnectorMandateId",
                        //                 )
                        //                 .change_context(errors::ApiErrorResponse::MandateDeserializationFailed)
                        //         })
                        //         .transpose()?
                        //         .map_or(
                        //             new_mandate_data.network_transaction_id.clone().map(|id| {
                        //                 api_models::payments::MandateReferenceId::NetworkMandateId(
                        //                     id,
                        //                 )
                        //             }),
                        //             |connector_id| Some(api_models::payments::MandateReferenceId::ConnectorMandateId(
                        //                 api_models::payments::ConnectorMandateReferenceId {
                        //                     connector_mandate_id: connector_id.connector_mandate_id,
                        //                     payment_method_id: connector_id.payment_method_id,
                        //                     update_history:None,
                        //
                        //                 }
                        //             )))
                        //     }));

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
    Ok(())
}

#[instrument(skip(state))]
pub async fn retrieve_mandates_list(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    constraints: api_models::mandates::MandateListConstraints,
) -> RouterResponse<Vec<api_models::mandates::MandateResponse>> {
    let mandates = state
        .store
        .as_ref()
        .find_mandates_by_merchant_id(&merchant_account.merchant_id, constraints)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to retrieve mandates")?;
    let mandates_list = future::try_join_all(mandates.into_iter().map(|mandate| {
        mandates::MandateResponse::from_db_mandate(&state, key_store.clone(), mandate)
    }))
    .await?;
    Ok(services::ApplicationResponse::Json(mandates_list))
}

impl ForeignFrom<Result<types::PaymentsResponseData, types::ErrorResponse>>
    for Option<types::MandateReference>
{
    fn foreign_from(resp: Result<types::PaymentsResponseData, types::ErrorResponse>) -> Self {
        match resp {
            Ok(types::PaymentsResponseData::TransactionResponse {
                mandate_reference, ..
            }) => mandate_reference,
            _ => None,
        }
    }
}

pub trait MandateBehaviour {
    fn get_amount(&self) -> i64;
    fn get_setup_future_usage(&self) -> Option<diesel_models::enums::FutureUsage>;
    fn get_mandate_id(&self) -> Option<&api_models::payments::MandateIds>;
    fn set_mandate_id(&mut self, new_mandate_id: Option<api_models::payments::MandateIds>);
    fn get_payment_method_data(&self) -> domain::payments::PaymentMethodData;
    fn get_setup_mandate_details(&self) -> Option<&data_models::mandates::MandateData>;
    fn get_customer_acceptance(&self) -> Option<api_models::payments::CustomerAcceptance>;
}
