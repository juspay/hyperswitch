use std::{str::FromStr, vec::IntoIter};

use common_utils::ext_traits::Encode;
use diesel_models::enums as storage_enums;
use error_stack::{IntoReport, ResultExt};
use router_env::{
    logger,
    tracing::{self, instrument},
};

use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payment_methods::PaymentMethodRetrieve,
        payments::{
            self,
            flows::{ConstructFlowSpecificData, Feature},
            operations,
        },
    },
    db::StorageInterface,
    routes,
    routes::{app, metrics},
    services, types,
    types::{api, domain, storage},
    utils,
};

#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn do_gsm_actions<F, ApiRequest, FData, Ctx>(
    state: &app::AppState,
    payment_data: &mut payments::PaymentData<F>,
    mut connectors: IntoIter<api::ConnectorData>,
    original_connector_data: api::ConnectorData,
    mut router_data: types::RouterData<F, FData, types::PaymentsResponseData>,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    operation: &operations::BoxedOperation<'_, F, ApiRequest, Ctx>,
    customer: &Option<domain::Customer>,
    validate_result: &operations::ValidateResult<'_>,
    schedule_time: Option<time::PrimitiveDateTime>,
    frm_suggestion: Option<storage_enums::FrmSuggestion>,
) -> RouterResult<types::RouterData<F, FData, types::PaymentsResponseData>>
where
    F: Clone + Send + Sync,
    FData: Send + Sync,
    payments::PaymentResponse: operations::Operation<F, FData, Ctx>,

    payments::PaymentData<F>: ConstructFlowSpecificData<F, FData, types::PaymentsResponseData>,
    types::RouterData<F, FData, types::PaymentsResponseData>: Feature<F, FData>,
    dyn api::Connector: services::api::ConnectorIntegration<F, FData, types::PaymentsResponseData>,
    Ctx: PaymentMethodRetrieve,
{
    let mut retries = None;

    metrics::AUTO_RETRY_ELIGIBLE_REQUEST_COUNT.add(&metrics::CONTEXT, 1, &[]);

    let mut initial_gsm = get_gsm(state, &router_data).await?;

    //Check if step-up to threeDS is possible and merchant has enabled
    let step_up_possible = initial_gsm
        .clone()
        .map(|gsm| gsm.step_up_possible)
        .unwrap_or(false);
    let is_no_three_ds_payment = matches!(
        payment_data.payment_attempt.authentication_type,
        Some(storage_enums::AuthenticationType::NoThreeDs)
    );
    let should_step_up = if step_up_possible && is_no_three_ds_payment {
        is_step_up_enabled_for_merchant_connector(
            state,
            &merchant_account.merchant_id,
            original_connector_data.connector_name,
        )
        .await
    } else {
        false
    };

    if should_step_up {
        router_data = do_retry(
            &state.clone(),
            original_connector_data,
            operation,
            customer,
            merchant_account,
            key_store,
            payment_data,
            router_data,
            validate_result,
            schedule_time,
            true,
            frm_suggestion,
        )
        .await?;
    }
    // Step up is not applicable so proceed with auto retries flow
    else {
        loop {
            // Use initial_gsm for first time alone
            let gsm = match initial_gsm.as_ref() {
                Some(gsm) => Some(gsm.clone()),
                None => get_gsm(state, &router_data).await?,
            };

            match get_gsm_decision(gsm) {
                api_models::gsm::GsmDecision::Retry => {
                    retries = get_retries(state, retries, &merchant_account.merchant_id).await;

                    if retries.is_none() || retries == Some(0) {
                        metrics::AUTO_RETRY_EXHAUSTED_COUNT.add(&metrics::CONTEXT, 1, &[]);
                        logger::info!("retries exhausted for auto_retry payment");
                        break;
                    }

                    if connectors.len() == 0 {
                        logger::info!("connectors exhausted for auto_retry payment");
                        metrics::AUTO_RETRY_EXHAUSTED_COUNT.add(&metrics::CONTEXT, 1, &[]);
                        break;
                    }

                    let connector = super::get_connector_data(&mut connectors)?;

                    router_data = do_retry(
                        &state.clone(),
                        connector,
                        operation,
                        customer,
                        merchant_account,
                        key_store,
                        payment_data,
                        router_data,
                        validate_result,
                        schedule_time,
                        //this is an auto retry payment, but not step-up
                        false,
                        frm_suggestion,
                    )
                    .await?;

                    retries = retries.map(|i| i - 1);
                }
                api_models::gsm::GsmDecision::Requeue => {
                    Err(errors::ApiErrorResponse::NotImplemented {
                        message: errors::api_error_response::NotImplementedMessage::Reason(
                            "Requeue not implemented".to_string(),
                        ),
                    })
                    .into_report()?
                }
                api_models::gsm::GsmDecision::DoDefault => break,
            }
            initial_gsm = None;
        }
    }
    Ok(router_data)
}

#[instrument(skip_all)]
pub async fn is_step_up_enabled_for_merchant_connector(
    state: &app::AppState,
    merchant_id: &str,
    connector_name: types::Connector,
) -> bool {
    let key = format!("step_up_enabled_{merchant_id}");
    let db = &*state.store;
    db.find_config_by_key_unwrap_or(key.as_str(), Some("[]".to_string()))
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .and_then(|step_up_config| {
            serde_json::from_str::<Vec<types::Connector>>(&step_up_config.config)
                .into_report()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Step-up config parsing failed")
        })
        .map_err(|err| {
            logger::error!(step_up_config_error=?err);
        })
        .ok()
        .map(|connectors_enabled| connectors_enabled.contains(&connector_name))
        .unwrap_or(false)
}

#[instrument(skip_all)]
pub async fn get_retries(
    state: &app::AppState,
    retries: Option<i32>,
    merchant_id: &str,
) -> Option<i32> {
    match retries {
        Some(retries) => Some(retries),
        None => {
            let key = format!("max_auto_retries_enabled_{merchant_id}");
            let db = &*state.store;
            db.find_config_by_key(key.as_str())
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .and_then(|retries_config| {
                    retries_config
                        .config
                        .parse::<i32>()
                        .into_report()
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Retries config parsing failed")
                })
                .map_err(|err| {
                    logger::error!(retries_error=?err);
                    None::<i32>
                })
                .ok()
        }
    }
}

#[instrument(skip_all)]
pub async fn get_gsm<F, FData>(
    state: &app::AppState,
    router_data: &types::RouterData<F, FData, types::PaymentsResponseData>,
) -> RouterResult<Option<storage::gsm::GatewayStatusMap>> {
    let error_response = router_data.response.as_ref().err();
    let error_code = error_response.map(|err| err.code.to_owned());
    let error_message = error_response.map(|err| err.message.to_owned());
    let connector_name = router_data.connector.to_string();
    let flow = get_flow_name::<F>()?;
    Ok(
        payments::helpers::get_gsm_record(state, error_code, error_message, connector_name, flow)
            .await,
    )
}

#[instrument(skip_all)]
pub fn get_gsm_decision(
    option_gsm: Option<storage::gsm::GatewayStatusMap>,
) -> api_models::gsm::GsmDecision {
    let option_gsm_decision = option_gsm
            .and_then(|gsm| {
                api_models::gsm::GsmDecision::from_str(gsm.decision.as_str())
                    .into_report()
                    .map_err(|err| {
                        let api_error = err.change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("gsm decision parsing failed");
                        logger::warn!(get_gsm_decision_parse_error=?api_error, "error fetching gsm decision");
                        api_error
                    })
                    .ok()
            });

    if option_gsm_decision.is_some() {
        metrics::AUTO_RETRY_GSM_MATCH_COUNT.add(&metrics::CONTEXT, 1, &[]);
    }
    option_gsm_decision.unwrap_or_default()
}

#[inline]
fn get_flow_name<F>() -> RouterResult<String> {
    Ok(std::any::type_name::<F>()
        .to_string()
        .rsplit("::")
        .next()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .into_report()
        .attach_printable("Flow stringify failed")?
        .to_string())
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn do_retry<F, ApiRequest, FData, Ctx>(
    state: &routes::AppState,
    connector: api::ConnectorData,
    operation: &operations::BoxedOperation<'_, F, ApiRequest, Ctx>,
    customer: &Option<domain::Customer>,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    payment_data: &mut payments::PaymentData<F>,
    router_data: types::RouterData<F, FData, types::PaymentsResponseData>,
    validate_result: &operations::ValidateResult<'_>,
    schedule_time: Option<time::PrimitiveDateTime>,
    is_step_up: bool,
    frm_suggestion: Option<storage_enums::FrmSuggestion>,
) -> RouterResult<types::RouterData<F, FData, types::PaymentsResponseData>>
where
    F: Clone + Send + Sync,
    FData: Send + Sync,
    payments::PaymentResponse: operations::Operation<F, FData, Ctx>,

    payments::PaymentData<F>: ConstructFlowSpecificData<F, FData, types::PaymentsResponseData>,
    types::RouterData<F, FData, types::PaymentsResponseData>: Feature<F, FData>,
    dyn api::Connector: services::api::ConnectorIntegration<F, FData, types::PaymentsResponseData>,
    Ctx: PaymentMethodRetrieve,
{
    metrics::AUTO_RETRY_PAYMENT_COUNT.add(&metrics::CONTEXT, 1, &[]);

    modify_trackers(
        state,
        connector.connector_name.to_string(),
        payment_data,
        merchant_account.storage_scheme,
        router_data,
        is_step_up,
    )
    .await?;

    payments::call_connector_service(
        state,
        merchant_account,
        key_store,
        connector,
        operation,
        payment_data,
        customer,
        payments::CallConnectorAction::Trigger,
        validate_result,
        schedule_time,
        api::HeaderPayload::default(),
        frm_suggestion,
    )
    .await
}

#[instrument(skip_all)]
pub async fn modify_trackers<F, FData>(
    state: &routes::AppState,
    connector: String,
    payment_data: &mut payments::PaymentData<F>,
    storage_scheme: storage_enums::MerchantStorageScheme,
    router_data: types::RouterData<F, FData, types::PaymentsResponseData>,
    is_step_up: bool,
) -> RouterResult<()>
where
    F: Clone + Send,
    FData: Send,
{
    let new_attempt_count = payment_data.payment_intent.attempt_count + 1;
    let new_payment_attempt = make_new_payment_attempt(
        connector,
        payment_data.payment_attempt.clone(),
        new_attempt_count,
        is_step_up,
    );

    let db = &*state.store;

    match router_data.response {
        Ok(types::PaymentsResponseData::TransactionResponse {
            resource_id,
            connector_metadata,
            redirection_data,
            ..
        }) => {
            let encoded_data = payment_data.payment_attempt.encoded_data.clone();

            let authentication_data = redirection_data
                .as_ref()
                .map(Encode::encode_to_value)
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Could not parse the connector response")?;

            db.update_payment_attempt_with_attempt_id(
                payment_data.payment_attempt.clone(),
                storage::PaymentAttemptUpdate::ResponseUpdate {
                    status: router_data.status,
                    connector: None,
                    connector_transaction_id: match resource_id {
                        types::ResponseId::NoResponseId => None,
                        types::ResponseId::ConnectorTransactionId(id)
                        | types::ResponseId::EncodedData(id) => Some(id),
                    },
                    connector_response_reference_id: payment_data
                        .payment_attempt
                        .connector_response_reference_id
                        .clone(),
                    authentication_type: None,
                    payment_method_id: Some(router_data.payment_method_id),
                    mandate_id: payment_data
                        .mandate_id
                        .clone()
                        .map(|mandate| mandate.mandate_id),
                    connector_metadata,
                    payment_token: None,
                    error_code: None,
                    error_message: None,
                    error_reason: None,
                    amount_capturable: if router_data.status.is_terminal_status() {
                        Some(0)
                    } else {
                        None
                    },
                    updated_by: storage_scheme.to_string(),
                    authentication_data,
                    encoded_data,
                    unified_code: None,
                    unified_message: None,
                },
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
        }
        Ok(_) => {
            logger::error!("unexpected response: this response was not expected in Retry flow");
            return Ok(());
        }
        Err(ref error_response) => {
            let option_gsm = get_gsm(state, &router_data).await?;
            db.update_payment_attempt_with_attempt_id(
                payment_data.payment_attempt.clone(),
                storage::PaymentAttemptUpdate::ErrorUpdate {
                    connector: None,
                    error_code: Some(Some(error_response.code.clone())),
                    error_message: Some(Some(error_response.message.clone())),
                    status: storage_enums::AttemptStatus::Failure,
                    error_reason: Some(error_response.reason.clone()),
                    amount_capturable: Some(0),
                    updated_by: storage_scheme.to_string(),
                    unified_code: option_gsm.clone().map(|gsm| gsm.unified_code),
                    unified_message: option_gsm.map(|gsm| gsm.unified_message),
                    connector_transaction_id: error_response.connector_transaction_id.clone(),
                },
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
        }
    }

    let payment_attempt = db
        .insert_payment_attempt(new_payment_attempt, storage_scheme)
        .await
        .to_duplicate_response(errors::ApiErrorResponse::DuplicatePayment {
            payment_id: payment_data.payment_intent.payment_id.clone(),
        })?;

    // update payment_attempt, connector_response and payment_intent in payment_data
    payment_data.payment_attempt = payment_attempt;

    payment_data.payment_intent = db
        .update_payment_intent(
            payment_data.payment_intent.clone(),
            storage::PaymentIntentUpdate::PaymentAttemptAndAttemptCountUpdate {
                active_attempt_id: payment_data.payment_attempt.attempt_id.clone(),
                attempt_count: new_attempt_count,
                updated_by: storage_scheme.to_string(),
            },
            storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    Ok(())
}

#[instrument(skip_all)]
pub fn make_new_payment_attempt(
    connector: String,
    old_payment_attempt: storage::PaymentAttempt,
    new_attempt_count: i16,
    is_step_up: bool,
) -> storage::PaymentAttemptNew {
    let created_at @ modified_at @ last_synced = Some(common_utils::date_time::now());
    storage::PaymentAttemptNew {
        connector: Some(connector),
        attempt_id: utils::get_payment_attempt_id(
            &old_payment_attempt.payment_id,
            new_attempt_count,
        ),
        payment_id: old_payment_attempt.payment_id,
        merchant_id: old_payment_attempt.merchant_id,
        status: old_payment_attempt.status,
        amount: old_payment_attempt.amount,
        currency: old_payment_attempt.currency,
        save_to_locker: old_payment_attempt.save_to_locker,

        offer_amount: old_payment_attempt.offer_amount,
        surcharge_amount: old_payment_attempt.surcharge_amount,
        tax_amount: old_payment_attempt.tax_amount,
        payment_method_id: old_payment_attempt.payment_method_id,
        payment_method: old_payment_attempt.payment_method,
        payment_method_type: old_payment_attempt.payment_method_type,
        capture_method: old_payment_attempt.capture_method,
        capture_on: old_payment_attempt.capture_on,
        confirm: old_payment_attempt.confirm,
        authentication_type: if is_step_up {
            Some(storage_enums::AuthenticationType::ThreeDs)
        } else {
            old_payment_attempt.authentication_type
        },

        amount_to_capture: old_payment_attempt.amount_to_capture,
        mandate_id: old_payment_attempt.mandate_id,
        browser_info: old_payment_attempt.browser_info,
        payment_token: old_payment_attempt.payment_token,

        created_at,
        modified_at,
        last_synced,
        ..storage::PaymentAttemptNew::default()
    }
}

pub async fn config_should_call_gsm(db: &dyn StorageInterface, merchant_id: &String) -> bool {
    let config = db
        .find_config_by_key_unwrap_or(
            format!("should_call_gsm_{}", merchant_id).as_str(),
            Some("false".to_string()),
        )
        .await;
    match config {
        Ok(conf) => conf.config == "true",
        Err(err) => {
            logger::error!("{err}");
            false
        }
    }
}

pub trait GsmValidation<F: Send + Clone + Sync, FData: Send + Sync, Resp> {
    // TODO : move this function to appropriate place later.
    fn should_call_gsm(&self) -> bool;
}

impl<F: Send + Clone + Sync, FData: Send + Sync>
    GsmValidation<F, FData, types::PaymentsResponseData>
    for types::RouterData<F, FData, types::PaymentsResponseData>
{
    #[inline(always)]
    fn should_call_gsm(&self) -> bool {
        if self.response.is_err() {
            true
        } else {
            match self.status {
                storage_enums::AttemptStatus::Started
                | storage_enums::AttemptStatus::AuthenticationPending
                | storage_enums::AttemptStatus::AuthenticationSuccessful
                | storage_enums::AttemptStatus::Authorized
                | storage_enums::AttemptStatus::Charged
                | storage_enums::AttemptStatus::Authorizing
                | storage_enums::AttemptStatus::CodInitiated
                | storage_enums::AttemptStatus::Voided
                | storage_enums::AttemptStatus::VoidInitiated
                | storage_enums::AttemptStatus::CaptureInitiated
                | storage_enums::AttemptStatus::RouterDeclined
                | storage_enums::AttemptStatus::VoidFailed
                | storage_enums::AttemptStatus::AutoRefunded
                | storage_enums::AttemptStatus::CaptureFailed
                | storage_enums::AttemptStatus::PartialCharged
                | storage_enums::AttemptStatus::PartialChargedAndChargeable
                | storage_enums::AttemptStatus::Pending
                | storage_enums::AttemptStatus::PaymentMethodAwaited
                | storage_enums::AttemptStatus::ConfirmationAwaited
                | storage_enums::AttemptStatus::Unresolved
                | storage_enums::AttemptStatus::DeviceDataCollectionPending => false,

                storage_enums::AttemptStatus::AuthenticationFailed
                | storage_enums::AttemptStatus::AuthorizationFailed
                | storage_enums::AttemptStatus::Failure => true,
            }
        }
    }
}
