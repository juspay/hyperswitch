use common_utils::{ext_traits::ValueExt, pii};
use error_stack::{report, ResultExt};
use masking::ExposeInterface;

use super::helpers;
use crate::{
    core::{
        errors::{self, ConnectorErrorExt, RouterResult},
        mandate, payment_methods, payments,
    },
    logger,
    routes::{metrics, AppState},
    services,
    types::{
        self,
        api::{self, PaymentMethodCreateExt},
        domain,
        storage::enums as storage_enums,
    },
    utils::OptionExt,
};

pub async fn save_payment_method<F: Clone, FData>(
    state: &AppState,
    connector: &api::ConnectorData,
    resp: types::RouterData<F, FData, types::PaymentsResponseData>,
    maybe_customer: &Option<domain::Customer>,
    merchant_account: &domain::MerchantAccount,
    payment_method_type: Option<storage_enums::PaymentMethodType>,
) -> RouterResult<Option<String>>
where
    FData: mandate::MandateBehaviour,
{
    match resp.response {
        Ok(_) => {
            let db = &*state.store;
            let token_store = state
                .conf
                .tokenization
                .0
                .get(&connector.connector_name.to_string())
                .map(|token_filter| token_filter.long_lived_token)
                .unwrap_or(false);

            let connector_token = if token_store {
                let token = resp
                    .payment_method_token
                    .to_owned()
                    .get_required_value("payment_token")?;
                Some((connector, token))
            } else {
                None
            };

            let pm_id = if resp.request.get_setup_future_usage().is_some() {
                let customer = maybe_customer.to_owned().get_required_value("customer")?;
                let payment_method_create_request = helpers::get_payment_method_create_request(
                    Some(&resp.request.get_payment_method_data()),
                    Some(resp.payment_method),
                    payment_method_type,
                    &customer,
                )
                .await?;
                let merchant_id = &merchant_account.merchant_id;

                let locker_response = save_in_locker(
                    state,
                    merchant_account,
                    payment_method_create_request.to_owned(),
                )
                .await?;
                let is_duplicate = locker_response.1;

                if is_duplicate {
                    let existing_pm = db
                        .find_payment_method(&locker_response.0.payment_method_id)
                        .await;
                    match existing_pm {
                        Ok(pm) => {
                            let pm_metadata = create_payment_method_metadata(
                                pm.metadata.as_ref(),
                                connector_token,
                            )?;
                            if let Some(metadata) = pm_metadata {
                                payment_methods::cards::update_payment_method(db, pm, metadata)
                                    .await
                                    .change_context(errors::ApiErrorResponse::InternalServerError)
                                    .attach_printable("Failed to add payment method in db")?;
                            };
                        }
                        Err(error) => {
                            match error.current_context() {
                                errors::StorageError::DatabaseError(err) => match err
                                    .current_context()
                                {
                                    storage_models::errors::DatabaseError::NotFound => {
                                        let pm_metadata =
                                            create_payment_method_metadata(None, connector_token)?;
                                        payment_methods::cards::create_payment_method(
                                            db,
                                            &payment_method_create_request,
                                            &customer.customer_id,
                                            &locker_response.0.payment_method_id,
                                            merchant_id,
                                            pm_metadata,
                                        )
                                        .await
                                        .change_context(
                                            errors::ApiErrorResponse::InternalServerError,
                                        )
                                        .attach_printable("Failed to add payment method in db")
                                    }
                                    _ => {
                                        Err(report!(errors::ApiErrorResponse::InternalServerError)
                                            .attach_printable(
                                                "Database Error while finding payment method",
                                            ))
                                    }
                                },
                                _ => Err(report!(errors::ApiErrorResponse::InternalServerError)
                                    .attach_printable("Error while finding payment method")),
                            }?;
                        }
                    };
                } else {
                    let pm_metadata = create_payment_method_metadata(None, connector_token)?;
                    payment_methods::cards::create_payment_method(
                        db,
                        &payment_method_create_request,
                        &customer.customer_id,
                        &locker_response.0.payment_method_id,
                        merchant_id,
                        pm_metadata,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to add payment method in db")?;
                };
                Some(locker_response.0.payment_method_id)
            } else {
                None
            };
            Ok(pm_id)
        }
        Err(_) => Ok(None),
    }
}

pub async fn save_in_locker(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    payment_method_request: api::PaymentMethodCreate,
) -> RouterResult<(api_models::payment_methods::PaymentMethodResponse, bool)> {
    payment_method_request.validate()?;
    let merchant_id = &merchant_account.merchant_id;
    let customer_id = payment_method_request
        .customer_id
        .clone()
        .get_required_value("customer_id")?;
    match payment_method_request.card.clone() {
        Some(card) => payment_methods::cards::add_card_to_locker(
            state,
            payment_method_request,
            card,
            customer_id,
            merchant_account,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Add Card Failed"),
        None => {
            let pm_id = common_utils::generate_id(crate::consts::ID_LENGTH, "pm");
            let payment_method_response = api::PaymentMethodResponse {
                merchant_id: merchant_id.to_string(),
                customer_id: Some(customer_id),
                payment_method_id: pm_id,
                payment_method: payment_method_request.payment_method,
                payment_method_type: payment_method_request.payment_method_type,
                card: None,
                metadata: None,
                created: Some(common_utils::date_time::now()),
                recurring_enabled: false,           //[#219]
                installment_payment_enabled: false, //[#219]
                payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]), //[#219]
            };
            Ok((payment_method_response, false))
        }
    }
}

pub fn create_payment_method_metadata(
    metadata: Option<&pii::SecretSerdeValue>,
    connector_token: Option<(&api::ConnectorData, String)>,
) -> RouterResult<Option<serde_json::Value>> {
    let mut meta = match metadata {
        None => serde_json::Map::new(),
        Some(meta) => {
            let metadata = meta.clone().expose();
            let existing_metadata: serde_json::Map<String, serde_json::Value> = metadata
                .parse_value("Map<String, Value>")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to parse the metadata")?;
            existing_metadata
        }
    };
    Ok(connector_token.and_then(|connector_and_token| {
        meta.insert(
            connector_and_token.0.connector_name.to_string(),
            serde_json::Value::String(connector_and_token.1),
        )
    }))
}

pub async fn add_payment_method_token<F: Clone, T: Clone>(
    state: &AppState,
    connector: &api::ConnectorData,
    tokenization_action: &payments::TokenizationAction,
    router_data: &types::RouterData<F, T, types::PaymentsResponseData>,
    pm_token_request_data: types::PaymentMethodTokenizationData,
) -> RouterResult<Option<String>> {
    match tokenization_action {
        payments::TokenizationAction::TokenizeInConnector => {
            let connector_integration: services::BoxedConnectorIntegration<
                '_,
                api::PaymentMethodToken,
                types::PaymentMethodTokenizationData,
                types::PaymentsResponseData,
            > = connector.connector.get_connector_integration();

            let pm_token_response_data: Result<types::PaymentsResponseData, types::ErrorResponse> =
                Err(types::ErrorResponse::default());

            let pm_token_router_data = payments::helpers::router_data_type_conversion::<
                _,
                api::PaymentMethodToken,
                _,
                _,
                _,
                _,
            >(
                router_data.clone(),
                pm_token_request_data,
                pm_token_response_data,
            );
            let resp = services::execute_connector_processing_step(
                state,
                connector_integration,
                &pm_token_router_data,
                payments::CallConnectorAction::Trigger,
                None,
            )
            .await
            .to_payment_failed_response()?;

            metrics::CONNECTOR_PAYMENT_METHOD_TOKENIZATION.add(
                &metrics::CONTEXT,
                1,
                &[
                    metrics::request::add_attributes(
                        "connector",
                        connector.connector_name.to_string(),
                    ),
                    metrics::request::add_attributes(
                        "payment_method",
                        router_data.payment_method.to_string(),
                    ),
                ],
            );

            let pm_token = match resp.response {
                Ok(response) => match response {
                    types::PaymentsResponseData::TokenizationResponse { token } => Some(token),
                    _ => None,
                },
                Err(err) => {
                    logger::debug!(payment_method_tokenization_error=?err);
                    None
                }
            };
            Ok(pm_token)
        }
        _ => Ok(None),
    }
}
