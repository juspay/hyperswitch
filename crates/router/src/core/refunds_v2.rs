use std::{fmt::Debug, str::FromStr};

use api_models::{enums::Connector, refunds::RefundErrorDetails};
use common_utils::{id_type, types as common_utils_types};
use diesel_models::refund as diesel_refund;
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    refunds::RefundListConstraints,
    router_data::{ErrorResponse, RouterData},
    router_data_v2::RefundFlowData,
};
use hyperswitch_interfaces::{
    api::{Connector as ConnectorTrait, ConnectorIntegration},
    connector_integration_v2::{ConnectorIntegrationV2, ConnectorV2},
    integrity::{CheckIntegrity, FlowIntegrity, GetIntegrityObject},
};
use router_env::{instrument, tracing};

use crate::{
    consts,
    core::{
        errors::{self, ConnectorErrorExt, StorageErrorExt},
        payments::{self, access_token, helpers},
        utils::{self as core_utils, refunds_validator},
    },
    db, logger,
    routes::{metrics, SessionState},
    services,
    types::{
        self,
        api::{self, refunds},
        domain,
        storage::{self, enums},
        transformers::{ForeignFrom, ForeignTryFrom},
    },
    utils,
};

#[instrument(skip_all)]
pub async fn refund_create_core(
    state: SessionState,
    platform: domain::Platform,
    req: refunds::RefundsCreateRequest,
    global_refund_id: id_type::GlobalRefundId,
) -> errors::RouterResponse<refunds::RefundResponse> {
    let db = &*state.store;
    let (payment_intent, payment_attempt, amount);

    payment_intent = db
        .find_payment_intent_by_id(
            &req.payment_id,
            platform.get_processor().get_key_store(),
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    utils::when(
        !(payment_intent.status == enums::IntentStatus::Succeeded
            || payment_intent.status == enums::IntentStatus::PartiallyCaptured),
        || {
            Err(report!(errors::ApiErrorResponse::PaymentUnexpectedState {
                current_flow: "refund".into(),
                field_name: "status".into(),
                current_value: payment_intent.status.to_string(),
                states: "succeeded, partially_captured".to_string()
            })
            .attach_printable("unable to refund for a unsuccessful payment intent"))
        },
    )?;

    let captured_amount = payment_intent
        .amount_captured
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("amount captured is none in a successful payment")?;

    // Amount is not passed in request refer from payment intent.
    amount = req.amount.unwrap_or(captured_amount);

    utils::when(amount <= common_utils_types::MinorUnit::new(0), || {
        Err(report!(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "amount".to_string(),
            expected_format: "positive integer".to_string()
        })
        .attach_printable("amount less than or equal to zero"))
    })?;

    payment_attempt = db
        .find_payment_attempt_last_successful_or_partially_captured_attempt_by_payment_id(
            platform.get_processor().get_key_store(),
            &req.payment_id,
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::SuccessfulPaymentNotFound)?;

    tracing::Span::current().record("global_refund_id", global_refund_id.get_string_repr());

    let merchant_connector_details = req.merchant_connector_details.clone();

    Box::pin(validate_and_create_refund(
        &state,
        &platform,
        &payment_attempt,
        &payment_intent,
        amount,
        req,
        global_refund_id,
        merchant_connector_details,
    ))
    .await
    .map(services::ApplicationResponse::Json)
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn trigger_refund_to_gateway(
    state: &SessionState,
    refund: &diesel_refund::Refund,
    platform: &domain::Platform,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: &storage::PaymentIntent,
    return_raw_connector_response: Option<bool>,
) -> errors::RouterResult<(diesel_refund::Refund, Option<masking::Secret<String>>)> {
    let db = &*state.store;

    let mca_id = payment_attempt.get_attempt_merchant_connector_account_id()?;

    let storage_scheme = platform.get_processor().get_account().storage_scheme;

    let mca = db
        .find_merchant_connector_account_by_id(&mca_id, platform.get_processor().get_key_store())
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch merchant connector account")?;

    metrics::REFUND_COUNT.add(
        1,
        router_env::metric_attributes!(("connector", mca_id.get_string_repr().to_string())),
    );

    let connector_enum = mca.connector_name;

    let connector: api::ConnectorData = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_enum.to_string(),
        api::GetToken::Connector,
        Some(mca_id.clone()),
    )?;

    refunds_validator::validate_for_valid_refunds(payment_attempt, connector.connector_name)?;

    let merchant_connector_account =
        domain::MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(Box::new(mca));

    let mut router_data = core_utils::construct_refund_router_data(
        state,
        connector_enum,
        platform,
        payment_intent,
        payment_attempt,
        refund,
        &merchant_connector_account,
    )
    .await?;

    let add_access_token_result = Box::pin(access_token::add_access_token(
        state,
        &connector,
        &router_data,
        None,
    ))
    .await?;

    logger::debug!(refund_router_data=?router_data);

    access_token::update_router_data_with_access_token_result(
        &add_access_token_result,
        &mut router_data,
        &payments::CallConnectorAction::Trigger,
    );

    let connector_response = Box::pin(call_connector_service(
        state,
        &connector,
        add_access_token_result,
        router_data,
        return_raw_connector_response,
    ))
    .await;

    let refund_update = get_refund_update_object(
        state,
        &connector,
        &storage_scheme,
        platform,
        &connector_response,
    )
    .await;

    let response = match refund_update {
        Some(refund_update) => state
            .store
            .update_refund(
                refund.to_owned(),
                refund_update,
                platform.get_processor().get_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
            .attach_printable_lazy(|| {
                format!(
                    "Failed while updating refund: refund_id: {}",
                    refund.id.get_string_repr()
                )
            })?,
        None => refund.to_owned(),
    };
    let raw_connector_response = connector_response
        .as_ref()
        .ok()
        .and_then(|data| data.raw_connector_response.clone());
    // Implement outgoing webhooks here
    connector_response.to_refund_failed_response()?;
    Ok((response, raw_connector_response))
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn internal_trigger_refund_to_gateway(
    state: &SessionState,
    refund: &diesel_refund::Refund,
    platform: &domain::Platform,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: &storage::PaymentIntent,
    merchant_connector_details: common_types::domain::MerchantConnectorAuthDetails,
    return_raw_connector_response: Option<bool>,
) -> errors::RouterResult<(diesel_refund::Refund, Option<masking::Secret<String>>)> {
    let storage_scheme = platform.get_processor().get_account().storage_scheme;

    let routed_through = payment_attempt
        .connector
        .clone()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to retrieve connector from payment attempt")?;

    metrics::REFUND_COUNT.add(
        1,
        router_env::metric_attributes!(("connector", routed_through.clone())),
    );

    let connector_enum = merchant_connector_details.connector_name;

    let connector: api::ConnectorData = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_enum.to_string(),
        api::GetToken::Connector,
        None,
    )?;

    refunds_validator::validate_for_valid_refunds(payment_attempt, connector.connector_name)?;

    let merchant_connector_account =
        domain::MerchantConnectorAccountTypeDetails::MerchantConnectorDetails(
            merchant_connector_details,
        );

    let mut router_data = core_utils::construct_refund_router_data(
        state,
        connector_enum,
        platform,
        payment_intent,
        payment_attempt,
        refund,
        &merchant_connector_account,
    )
    .await?;

    let add_access_token_result = Box::pin(access_token::add_access_token(
        state,
        &connector,
        &router_data,
        None,
    ))
    .await?;

    access_token::update_router_data_with_access_token_result(
        &add_access_token_result,
        &mut router_data,
        &payments::CallConnectorAction::Trigger,
    );

    let connector_response = Box::pin(call_connector_service(
        state,
        &connector,
        add_access_token_result,
        router_data,
        return_raw_connector_response,
    ))
    .await;

    let refund_update = get_refund_update_object(
        state,
        &connector,
        &storage_scheme,
        platform,
        &connector_response,
    )
    .await;

    let response = match refund_update {
        Some(refund_update) => state
            .store
            .update_refund(
                refund.to_owned(),
                refund_update,
                platform.get_processor().get_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
            .attach_printable_lazy(|| {
                format!(
                    "Failed while updating refund: refund_id: {}",
                    refund.id.get_string_repr()
                )
            })?,
        None => refund.to_owned(),
    };
    let raw_connector_response = connector_response
        .as_ref()
        .ok()
        .and_then(|data| data.raw_connector_response.clone());
    // Implement outgoing webhooks here
    connector_response.to_refund_failed_response()?;
    Ok((response, raw_connector_response))
}

async fn call_connector_service<F>(
    state: &SessionState,
    connector: &api::ConnectorData,
    add_access_token_result: types::AddAccessTokenResult,
    router_data: RouterData<F, types::RefundsData, types::RefundsResponseData>,
    return_raw_connector_response: Option<bool>,
) -> Result<
    RouterData<F, types::RefundsData, types::RefundsResponseData>,
    error_stack::Report<errors::ConnectorError>,
>
where
    F: Debug + Clone + 'static,
    dyn ConnectorTrait + Sync:
        ConnectorIntegration<F, types::RefundsData, types::RefundsResponseData>,
    dyn ConnectorV2 + Sync:
        ConnectorIntegrationV2<F, RefundFlowData, types::RefundsData, types::RefundsResponseData>,
{
    if !(add_access_token_result.connector_supports_access_token
        && router_data.access_token.is_none())
    {
        let connector_integration: services::BoxedRefundConnectorIntegrationInterface<
            F,
            types::RefundsData,
            types::RefundsResponseData,
        > = connector.connector.get_connector_integration();
        services::execute_connector_processing_step(
            state,
            connector_integration,
            &router_data,
            payments::CallConnectorAction::Trigger,
            None,
            return_raw_connector_response,
        )
        .await
    } else {
        Ok(router_data)
    }
}

async fn get_refund_update_object(
    state: &SessionState,
    connector: &api::ConnectorData,
    storage_scheme: &enums::MerchantStorageScheme,
    platform: &domain::Platform,
    router_data_response: &Result<
        RouterData<api::Execute, types::RefundsData, types::RefundsResponseData>,
        error_stack::Report<errors::ConnectorError>,
    >,
) -> Option<diesel_refund::RefundUpdate> {
    match router_data_response {
        // This error is related to connector implementation i.e if no implementation for refunds for that specific connector in HS or the connector does not support refund itself.
        Err(err) => get_connector_implementation_error_refund_update(err, *storage_scheme),
        Ok(response) => {
            let response = perform_integrity_check(response.clone());
            match response.response.clone() {
                Err(err) => Some(
                    get_connector_error_refund_update(state, err, connector, storage_scheme).await,
                ),
                Ok(refund_response_data) => Some(get_refund_update_for_refund_response_data(
                    response,
                    connector,
                    refund_response_data,
                    storage_scheme,
                    platform,
                )),
            }
        }
    }
}

fn get_connector_implementation_error_refund_update(
    error: &error_stack::Report<errors::ConnectorError>,
    storage_scheme: enums::MerchantStorageScheme,
) -> Option<diesel_refund::RefundUpdate> {
    Option::<diesel_refund::RefundUpdate>::foreign_from((error.current_context(), storage_scheme))
}

async fn get_connector_error_refund_update(
    state: &SessionState,
    err: ErrorResponse,
    connector: &api::ConnectorData,
    storage_scheme: &enums::MerchantStorageScheme,
) -> diesel_refund::RefundUpdate {
    let unified_error_object = get_unified_error_and_message(state, &err, connector).await;

    diesel_refund::RefundUpdate::build_error_update_for_unified_error_and_message(
        unified_error_object,
        err.reason.or(Some(err.message)),
        Some(err.code),
        storage_scheme,
    )
}

async fn get_unified_error_and_message(
    state: &SessionState,
    err: &ErrorResponse,
    connector: &api::ConnectorData,
) -> (String, String) {
    let option_gsm = helpers::get_gsm_record(
        state,
        Some(err.code.clone()),
        Some(err.message.clone()),
        connector.connector_name.to_string(),
        consts::REFUND_FLOW_STR.to_string(),
    )
    .await;
    // Note: Some connectors do not have a separate list of refund errors
    // In such cases, the error codes and messages are stored under "Authorize" flow in GSM table
    // So we will have to fetch the GSM using Authorize flow in case GSM is not found using "refund_flow"
    let option_gsm = if option_gsm.is_none() {
        helpers::get_gsm_record(
            state,
            Some(err.code.clone()),
            Some(err.message.clone()),
            connector.connector_name.to_string(),
            consts::AUTHORIZE_FLOW_STR.to_string(),
        )
        .await
    } else {
        option_gsm
    };

    let gsm_unified_code = option_gsm.as_ref().and_then(|gsm| gsm.unified_code.clone());
    let gsm_unified_message = option_gsm.and_then(|gsm| gsm.unified_message);

    match gsm_unified_code.as_ref().zip(gsm_unified_message.as_ref()) {
        Some((code, message)) => (code.to_owned(), message.to_owned()),
        None => (
            consts::DEFAULT_UNIFIED_ERROR_CODE.to_owned(),
            consts::DEFAULT_UNIFIED_ERROR_MESSAGE.to_owned(),
        ),
    }
}

pub fn get_refund_update_for_refund_response_data(
    router_data: RouterData<api::Execute, types::RefundsData, types::RefundsResponseData>,
    connector: &api::ConnectorData,
    refund_response_data: types::RefundsResponseData,
    storage_scheme: &enums::MerchantStorageScheme,
    platform: &domain::Platform,
) -> diesel_refund::RefundUpdate {
    // match on connector integrity checks
    match router_data.integrity_check.clone() {
        Err(err) => {
            let connector_refund_id = err
                .connector_transaction_id
                .map(common_utils_types::ConnectorTransactionId::from);

            metrics::INTEGRITY_CHECK_FAILED.add(
                1,
                router_env::metric_attributes!(
                    ("connector", connector.connector_name.to_string()),
                    (
                        "merchant_id",
                        platform.get_processor().get_account().get_id().clone()
                    ),
                ),
            );

            diesel_refund::RefundUpdate::build_error_update_for_integrity_check_failure(
                err.field_names,
                connector_refund_id,
                storage_scheme,
            )
        }
        Ok(()) => {
            if refund_response_data.refund_status == diesel_models::enums::RefundStatus::Success {
                metrics::SUCCESSFUL_REFUND.add(
                    1,
                    router_env::metric_attributes!((
                        "connector",
                        connector.connector_name.to_string(),
                    )),
                )
            }

            let connector_refund_id = common_utils_types::ConnectorTransactionId::from(
                refund_response_data.connector_refund_id,
            );

            diesel_refund::RefundUpdate::build_refund_update(
                connector_refund_id,
                refund_response_data.refund_status,
                storage_scheme,
            )
        }
    }
}

pub fn perform_integrity_check<F>(
    mut router_data: RouterData<F, types::RefundsData, types::RefundsResponseData>,
) -> RouterData<F, types::RefundsData, types::RefundsResponseData>
where
    F: Debug + Clone + 'static,
{
    // Initiating Integrity check
    let integrity_result = check_refund_integrity(&router_data.request, &router_data.response);
    router_data.integrity_check = integrity_result;
    router_data
}

impl ForeignFrom<(&errors::ConnectorError, enums::MerchantStorageScheme)>
    for Option<diesel_refund::RefundUpdate>
{
    fn foreign_from(
        (from, storage_scheme): (&errors::ConnectorError, enums::MerchantStorageScheme),
    ) -> Self {
        match from {
            errors::ConnectorError::NotImplemented(message) => {
                Some(diesel_refund::RefundUpdate::ErrorUpdate {
                    refund_status: Some(enums::RefundStatus::Failure),
                    refund_error_message: Some(
                        errors::ConnectorError::NotImplemented(message.to_owned()).to_string(),
                    ),
                    refund_error_code: Some("NOT_IMPLEMENTED".to_string()),
                    updated_by: storage_scheme.to_string(),
                    connector_refund_id: None,
                    processor_refund_data: None,
                    unified_code: None,
                    unified_message: None,
                })
            }
            errors::ConnectorError::NotSupported { message, connector } => {
                Some(diesel_refund::RefundUpdate::ErrorUpdate {
                    refund_status: Some(enums::RefundStatus::Failure),
                    refund_error_message: Some(format!(
                        "{message} is not supported by {connector}"
                    )),
                    refund_error_code: Some("NOT_SUPPORTED".to_string()),
                    updated_by: storage_scheme.to_string(),
                    connector_refund_id: None,
                    processor_refund_data: None,
                    unified_code: None,
                    unified_message: None,
                })
            }
            _ => None,
        }
    }
}

pub fn check_refund_integrity<T, Request>(
    request: &Request,
    refund_response_data: &Result<types::RefundsResponseData, ErrorResponse>,
) -> Result<(), common_utils::errors::IntegrityCheckError>
where
    T: FlowIntegrity,
    Request: GetIntegrityObject<T> + CheckIntegrity<Request, T>,
{
    let connector_refund_id = refund_response_data
        .as_ref()
        .map(|resp_data| resp_data.connector_refund_id.clone())
        .ok();

    request.check_integrity(request, connector_refund_id.to_owned())
}

// ********************************************** REFUND UPDATE **********************************************

pub async fn refund_metadata_update_core(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    req: refunds::RefundMetadataUpdateRequest,
    global_refund_id: id_type::GlobalRefundId,
) -> errors::RouterResponse<refunds::RefundResponse> {
    let db = state.store.as_ref();
    let refund = db
        .find_refund_by_id(&global_refund_id, merchant_account.storage_scheme)
        .await
        .to_not_found_response(errors::ApiErrorResponse::RefundNotFound)?;

    let response = db
        .update_refund(
            refund,
            diesel_refund::RefundUpdate::MetadataAndReasonUpdate {
                metadata: req.metadata,
                reason: req.reason,
                updated_by: merchant_account.storage_scheme.to_string(),
            },
            merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!(
                "Unable to update refund with refund_id: {}",
                global_refund_id.get_string_repr()
            )
        })?;

    refunds::RefundResponse::foreign_try_from(response).map(services::ApplicationResponse::Json)
}

// ********************************************** REFUND SYNC **********************************************

#[instrument(skip_all)]
pub async fn refund_retrieve_core_with_refund_id(
    state: SessionState,
    platform: domain::Platform,
    profile: domain::Profile,
    request: refunds::RefundsRetrieveRequest,
) -> errors::RouterResponse<refunds::RefundResponse> {
    let refund_id = request.refund_id.clone();
    let db = &*state.store;
    let profile_id = profile.get_id().to_owned();
    let refund = db
        .find_refund_by_id(
            &refund_id,
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::RefundNotFound)?;

    let (response, raw_connector_response) = Box::pin(refund_retrieve_core(
        state.clone(),
        platform,
        Some(profile_id),
        request,
        refund,
    ))
    .await?;

    api::RefundResponse::foreign_try_from(response).map(services::ApplicationResponse::Json)
}

#[instrument(skip_all)]
pub async fn refund_retrieve_core(
    state: SessionState,
    platform: domain::Platform,
    profile_id: Option<id_type::ProfileId>,
    request: refunds::RefundsRetrieveRequest,
    refund: diesel_refund::Refund,
) -> errors::RouterResult<(diesel_refund::Refund, Option<masking::Secret<String>>)> {
    let db = &*state.store;

    core_utils::validate_profile_id_from_auth_layer(profile_id, &refund)?;
    let payment_id = &refund.payment_id;
    let payment_intent = db
        .find_payment_intent_by_id(
            payment_id,
            platform.get_processor().get_key_store(),
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    let active_attempt_id = payment_intent
        .active_attempt_id
        .clone()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Active attempt id not found")?;

    let payment_attempt = db
        .find_payment_attempt_by_id(
            platform.get_processor().get_key_store(),
            &active_attempt_id,
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::InternalServerError)?;

    let unified_translated_message = if let (Some(unified_code), Some(unified_message)) =
        (refund.unified_code.clone(), refund.unified_message.clone())
    {
        helpers::get_unified_translation(
            &state,
            unified_code,
            unified_message.clone(),
            state.locale.to_string(),
        )
        .await
        .or(Some(unified_message))
    } else {
        refund.unified_message
    };

    let refund = diesel_refund::Refund {
        unified_message: unified_translated_message,
        ..refund
    };

    let (response, raw_connector_response) = if should_call_refund(
        &refund,
        request.force_sync.unwrap_or(false),
        request.return_raw_connector_response.unwrap_or(false),
    ) {
        if state.conf.merchant_id_auth.merchant_id_auth_enabled {
            let merchant_connector_details = match request.merchant_connector_details {
                Some(details) => details,
                None => {
                    return Err(report!(errors::ApiErrorResponse::MissingRequiredField {
                        field_name: "merchant_connector_details"
                    }));
                }
            };
            Box::pin(internal_sync_refund_with_gateway(
                &state,
                &platform,
                &payment_attempt,
                &payment_intent,
                &refund,
                merchant_connector_details,
                request.return_raw_connector_response,
            ))
            .await
        } else {
            Box::pin(sync_refund_with_gateway(
                &state,
                &platform,
                &payment_attempt,
                &payment_intent,
                &refund,
                request.return_raw_connector_response,
            ))
            .await
        }
    } else {
        Ok((refund, None))
    }?;
    Ok((response, raw_connector_response))
}

fn should_call_refund(
    refund: &diesel_models::refund::Refund,
    force_sync: bool,
    return_raw_connector_response: bool,
) -> bool {
    // This implies, we cannot perform a refund sync & `the connector_refund_id`
    // doesn't exist
    let predicate1 = refund.connector_refund_id.is_some();

    // This allows refund sync at connector level if force_sync is enabled, or
    // checks if the refund has failed
    let predicate2 = return_raw_connector_response
        || force_sync
        || !matches!(
            refund.refund_status,
            diesel_models::enums::RefundStatus::Failure
                | diesel_models::enums::RefundStatus::Success
        );

    predicate1 && predicate2
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn sync_refund_with_gateway(
    state: &SessionState,
    platform: &domain::Platform,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: &storage::PaymentIntent,
    refund: &diesel_refund::Refund,
    return_raw_connector_response: Option<bool>,
) -> errors::RouterResult<(diesel_refund::Refund, Option<masking::Secret<String>>)> {
    let db = &*state.store;

    let connector_id = refund.connector.to_string();
    let connector: api::ConnectorData = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_id,
        api::GetToken::Connector,
        payment_attempt.merchant_connector_id.clone(),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get the connector")?;

    let mca_id = payment_attempt.get_attempt_merchant_connector_account_id()?;

    let mca = db
        .find_merchant_connector_account_by_id(&mca_id, platform.get_processor().get_key_store())
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch merchant connector account")?;

    let connector_enum = mca.connector_name;

    let merchant_connector_account =
        domain::MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(Box::new(mca));

    let mut router_data = core_utils::construct_refund_router_data::<api::RSync>(
        state,
        connector_enum,
        platform,
        payment_intent,
        payment_attempt,
        refund,
        &merchant_connector_account,
    )
    .await?;

    let add_access_token_result = Box::pin(access_token::add_access_token(
        state,
        &connector,
        &router_data,
        None,
    ))
    .await?;

    logger::debug!(refund_retrieve_router_data=?router_data);

    access_token::update_router_data_with_access_token_result(
        &add_access_token_result,
        &mut router_data,
        &payments::CallConnectorAction::Trigger,
    );

    let connector_response = Box::pin(call_connector_service(
        state,
        &connector,
        add_access_token_result,
        router_data,
        return_raw_connector_response,
    ))
    .await
    .to_refund_failed_response()?;

    let connector_response = perform_integrity_check(connector_response);

    let refund_update =
        build_refund_update_for_rsync(&connector, platform, connector_response.clone());

    let response = state
        .store
        .update_refund(
            refund.to_owned(),
            refund_update,
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::RefundNotFound)
        .attach_printable_lazy(|| {
            format!(
                "Unable to update refund with refund_id: {}",
                refund.id.get_string_repr()
            )
        })?;

    // Implement outgoing webhook here
    Ok((response, connector_response.raw_connector_response))
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn internal_sync_refund_with_gateway(
    state: &SessionState,
    platform: &domain::Platform,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: &storage::PaymentIntent,
    refund: &diesel_refund::Refund,
    merchant_connector_details: common_types::domain::MerchantConnectorAuthDetails,
    return_raw_connector_response: Option<bool>,
) -> errors::RouterResult<(diesel_refund::Refund, Option<masking::Secret<String>>)> {
    let connector_enum = merchant_connector_details.connector_name;

    let connector: api::ConnectorData = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_enum.to_string(),
        api::GetToken::Connector,
        None,
    )?;

    let merchant_connector_account =
        domain::MerchantConnectorAccountTypeDetails::MerchantConnectorDetails(
            merchant_connector_details,
        );

    let mut router_data = core_utils::construct_refund_router_data::<api::RSync>(
        state,
        connector_enum,
        platform,
        payment_intent,
        payment_attempt,
        refund,
        &merchant_connector_account,
    )
    .await?;

    let add_access_token_result = Box::pin(access_token::add_access_token(
        state,
        &connector,
        &router_data,
        None,
    ))
    .await?;

    access_token::update_router_data_with_access_token_result(
        &add_access_token_result,
        &mut router_data,
        &payments::CallConnectorAction::Trigger,
    );

    let connector_response = Box::pin(call_connector_service(
        state,
        &connector,
        add_access_token_result,
        router_data,
        return_raw_connector_response,
    ))
    .await
    .to_refund_failed_response()?;

    let connector_response = perform_integrity_check(connector_response);

    let refund_update =
        build_refund_update_for_rsync(&connector, platform, connector_response.clone());

    let response = state
        .store
        .update_refund(
            refund.to_owned(),
            refund_update,
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::RefundNotFound)
        .attach_printable_lazy(|| {
            format!(
                "Unable to update refund with refund_id: {}",
                refund.id.get_string_repr()
            )
        })?;

    // Implement outgoing webhook here
    Ok((response, connector_response.raw_connector_response))
}

pub fn build_refund_update_for_rsync(
    connector: &api::ConnectorData,
    platform: &domain::Platform,
    router_data_response: RouterData<api::RSync, types::RefundsData, types::RefundsResponseData>,
) -> diesel_refund::RefundUpdate {
    let merchant_account = platform.get_processor().get_account();
    let storage_scheme = &platform.get_processor().get_account().storage_scheme;

    match router_data_response.response {
        Err(error_message) => {
            let refund_status = match error_message.status_code {
                // marking failure for 2xx because this is genuine refund failure
                200..=299 => Some(enums::RefundStatus::Failure),
                _ => None,
            };
            let refund_error_message = error_message.reason.or(Some(error_message.message));
            let refund_error_code = Some(error_message.code);

            diesel_refund::RefundUpdate::build_error_update_for_refund_failure(
                refund_status,
                refund_error_message,
                refund_error_code,
                storage_scheme,
            )
        }
        Ok(response) => match router_data_response.integrity_check.clone() {
            Err(err) => {
                metrics::INTEGRITY_CHECK_FAILED.add(
                    1,
                    router_env::metric_attributes!(
                        ("connector", connector.connector_name.to_string()),
                        ("merchant_id", merchant_account.get_id().clone()),
                    ),
                );

                let connector_refund_id = err
                    .connector_transaction_id
                    .map(common_utils_types::ConnectorTransactionId::from);

                diesel_refund::RefundUpdate::build_error_update_for_integrity_check_failure(
                    err.field_names,
                    connector_refund_id,
                    storage_scheme,
                )
            }
            Ok(()) => {
                let connector_refund_id =
                    common_utils_types::ConnectorTransactionId::from(response.connector_refund_id);

                diesel_refund::RefundUpdate::build_refund_update(
                    connector_refund_id,
                    response.refund_status,
                    storage_scheme,
                )
            }
        },
    }
}

// ********************************************** Refund list **********************************************

///   If payment_id is provided, lists all the refunds associated with that particular payment_id
///   If payment_id is not provided, lists the refunds associated with that particular merchant - to the limit specified,if no limits given, it is 10 by default
#[instrument(skip_all)]
#[cfg(feature = "olap")]
pub async fn refund_list(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    profile: domain::Profile,
    req: refunds::RefundListRequest,
) -> errors::RouterResponse<refunds::RefundListResponse> {
    let db = state.store;
    let limit = refunds_validator::validate_refund_list(req.limit)?;
    let offset = req.offset.unwrap_or_default();

    let refund_list = db
        .filter_refund_by_constraints(
            merchant_account.get_id(),
            RefundListConstraints::from((req.clone(), profile.clone())),
            merchant_account.storage_scheme,
            limit,
            offset,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::RefundNotFound)?;

    let data: Vec<refunds::RefundResponse> = refund_list
        .into_iter()
        .map(refunds::RefundResponse::foreign_try_from)
        .collect::<Result<_, _>>()?;

    let total_count = db
        .get_total_count_of_refunds(
            merchant_account.get_id(),
            RefundListConstraints::from((req, profile)),
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::InternalServerError)?;

    Ok(services::ApplicationResponse::Json(
        api_models::refunds::RefundListResponse {
            count: data.len(),
            total_count,
            data,
        },
    ))
}

// ********************************************** VALIDATIONS **********************************************

#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn validate_and_create_refund(
    state: &SessionState,
    platform: &domain::Platform,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: &storage::PaymentIntent,
    refund_amount: common_utils_types::MinorUnit,
    req: refunds::RefundsCreateRequest,
    global_refund_id: id_type::GlobalRefundId,
    merchant_connector_details: Option<common_types::domain::MerchantConnectorAuthDetails>,
) -> errors::RouterResult<refunds::RefundResponse> {
    let db = &*state.store;

    let refund_type = req.refund_type.unwrap_or_default();

    let merchant_reference_id = req.merchant_reference_id;

    let predicate = req
        .merchant_id
        .as_ref()
        .map(|merchant_id| merchant_id != platform.get_processor().get_account().get_id());

    utils::when(predicate.unwrap_or(false), || {
        Err(report!(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "merchant_id".to_string(),
            expected_format: "merchant_id from merchant account".to_string()
        })
        .attach_printable("invalid merchant_id in request"))
    })?;

    let connector_payment_id = payment_attempt.clone().connector_payment_id.ok_or_else(|| {
        report!(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Transaction in invalid. Missing field \"connector_transaction_id\" in payment_attempt.")
    })?;

    let all_refunds = db
        .find_refund_by_merchant_id_connector_transaction_id(
            platform.get_processor().get_account().get_id(),
            &connector_payment_id,
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::RefundNotFound)?;

    let currency = payment_intent.amount_details.currency;

    refunds_validator::validate_payment_order_age(
        &payment_intent.created_at,
        state.conf.refund.max_age,
    )
    .change_context(errors::ApiErrorResponse::InvalidDataFormat {
        field_name: "created_at".to_string(),
        expected_format: format!(
            "created_at not older than {} days",
            state.conf.refund.max_age,
        ),
    })?;

    let total_amount_captured = payment_intent
        .amount_captured
        .unwrap_or(payment_attempt.get_total_amount());

    refunds_validator::validate_refund_amount(
        total_amount_captured.get_amount_as_i64(),
        &all_refunds,
        refund_amount.get_amount_as_i64(),
    )
    .change_context(errors::ApiErrorResponse::RefundAmountExceedsPaymentAmount)?;

    refunds_validator::validate_maximum_refund_against_payment_attempt(
        &all_refunds,
        state.conf.refund.max_attempts,
    )
    .change_context(errors::ApiErrorResponse::MaximumRefundCount)?;

    let connector = payment_attempt
        .connector
        .clone()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("No connector populated in payment attempt")?;
    let (connector_transaction_id, processor_transaction_data) =
        common_utils_types::ConnectorTransactionId::form_id_and_data(connector_payment_id);
    let refund_create_req = diesel_refund::RefundNew {
        id: global_refund_id,
        merchant_reference_id: merchant_reference_id.clone(),
        external_reference_id: Some(merchant_reference_id.get_string_repr().to_string()),
        payment_id: req.payment_id,
        merchant_id: platform.get_processor().get_account().get_id().clone(),
        connector_transaction_id,
        connector,
        refund_type: enums::RefundType::foreign_from(req.refund_type.unwrap_or_default()),
        total_amount: payment_attempt.get_total_amount(),
        refund_amount,
        currency,
        created_at: common_utils::date_time::now(),
        modified_at: common_utils::date_time::now(),
        refund_status: enums::RefundStatus::Pending,
        metadata: req.metadata,
        description: req.reason.clone(),
        attempt_id: payment_attempt.id.clone(),
        refund_reason: req.reason,
        profile_id: Some(payment_intent.profile_id.clone()),
        connector_id: payment_attempt.merchant_connector_id.clone(),
        charges: None,
        split_refunds: None,
        connector_refund_id: None,
        sent_to_gateway: Default::default(),
        refund_arn: None,
        updated_by: Default::default(),
        organization_id: platform
            .get_processor()
            .get_account()
            .organization_id
            .clone(),
        processor_transaction_data,
        processor_refund_data: None,
    };

    let (refund, raw_connector_response) = match db
        .insert_refund(
            refund_create_req,
            platform.get_processor().get_account().storage_scheme,
        )
        .await
    {
        Ok(refund) => {
            Box::pin(schedule_refund_execution(
                state,
                refund.clone(),
                refund_type,
                platform,
                payment_attempt,
                payment_intent,
                merchant_connector_details,
                req.return_raw_connector_response,
            ))
            .await?
        }
        Err(err) => {
            if err.current_context().is_db_unique_violation() {
                Err(errors::ApiErrorResponse::DuplicateRefundRequest)?
            } else {
                Err(err)
                    .change_context(errors::ApiErrorResponse::RefundFailed { data: None })
                    .attach_printable("Failed to insert refund")?
            }
        }
    };

    let unified_translated_message =
        match (refund.unified_code.clone(), refund.unified_message.clone()) {
            (Some(unified_code), Some(unified_message)) => helpers::get_unified_translation(
                state,
                unified_code,
                unified_message.clone(),
                state.locale.to_string(),
            )
            .await
            .or(Some(unified_message)),
            _ => refund.unified_message,
        };

    let refund = diesel_refund::Refund {
        unified_message: unified_translated_message,
        ..refund
    };

    let mut refund_response: api::RefundResponse = api::RefundResponse::foreign_try_from(refund)?;
    refund_response.raw_connector_response = raw_connector_response;

    Ok(refund_response)
}

impl ForeignTryFrom<diesel_refund::Refund> for api::RefundResponse {
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn foreign_try_from(refund: diesel_refund::Refund) -> Result<Self, Self::Error> {
        let refund = refund;

        let profile_id = refund
            .profile_id
            .clone()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Profile id not found")?;

        let connector_name = refund.connector;
        let connector = Connector::from_str(&connector_name)
            .change_context(errors::ConnectorError::InvalidConnectorName)
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "connector",
            })
            .attach_printable_lazy(|| {
                format!("unable to parse connector name {connector_name:?}")
            })?;

        Ok(Self {
            payment_id: refund.payment_id,
            id: refund.id.clone(),
            amount: refund.refund_amount,
            currency: refund.currency,
            reason: refund.refund_reason,
            status: refunds::RefundStatus::foreign_from(refund.refund_status),
            profile_id,
            metadata: refund.metadata,
            created_at: refund.created_at,
            updated_at: refund.modified_at,
            connector,
            merchant_connector_id: refund.connector_id,
            merchant_reference_id: Some(refund.merchant_reference_id),
            error_details: Some(RefundErrorDetails {
                code: refund.refund_error_code.unwrap_or_default(),
                message: refund.refund_error_message.unwrap_or_default(),
            }),
            connector_refund_reference_id: None,
            raw_connector_response: None,
        })
    }
}

// ********************************************** PROCESS TRACKER **********************************************

#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn schedule_refund_execution(
    state: &SessionState,
    refund: diesel_refund::Refund,
    refund_type: api_models::refunds::RefundType,
    platform: &domain::Platform,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: &storage::PaymentIntent,
    merchant_connector_details: Option<common_types::domain::MerchantConnectorAuthDetails>,
    return_raw_connector_response: Option<bool>,
) -> errors::RouterResult<(diesel_refund::Refund, Option<masking::Secret<String>>)> {
    let db = &*state.store;
    let runner = storage::ProcessTrackerRunner::RefundWorkflowRouter;
    let task = "EXECUTE_REFUND";
    let task_id = format!("{runner}_{task}_{}", refund.id.get_string_repr());

    let refund_process = db
        .find_process_by_id(&task_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to find the process id")?;

    let result = match refund.refund_status {
        enums::RefundStatus::Pending | enums::RefundStatus::ManualReview => {
            match (refund.sent_to_gateway, refund_process) {
                (false, None) => {
                    // Execute the refund task based on refund_type
                    match refund_type {
                        api_models::refunds::RefundType::Scheduled => {
                            add_refund_execute_task(db, &refund, runner)
                                .await
                                .change_context(errors::ApiErrorResponse::InternalServerError)
                                .attach_printable_lazy(|| format!("Failed while pushing refund execute task to scheduler, refund_id: {}", refund.id.get_string_repr()))?;

                            Ok((refund, None))
                        }
                        api_models::refunds::RefundType::Instant => {
                            let update_refund =
                                if state.conf.merchant_id_auth.merchant_id_auth_enabled {
                                    let merchant_connector_details =
                                        match merchant_connector_details {
                                            Some(details) => details,
                                            None => {
                                                return Err(report!(
                                            errors::ApiErrorResponse::MissingRequiredField {
                                                field_name: "merchant_connector_details"
                                            }
                                        ));
                                            }
                                        };
                                    Box::pin(internal_trigger_refund_to_gateway(
                                        state,
                                        &refund,
                                        platform,
                                        payment_attempt,
                                        payment_intent,
                                        merchant_connector_details,
                                        return_raw_connector_response,
                                    ))
                                    .await
                                } else {
                                    Box::pin(trigger_refund_to_gateway(
                                        state,
                                        &refund,
                                        platform,
                                        payment_attempt,
                                        payment_intent,
                                        return_raw_connector_response,
                                    ))
                                    .await
                                };

                            match update_refund {
                                Ok((updated_refund_data, raw_connector_response)) => {
                                    add_refund_sync_task(db, &updated_refund_data, runner)
                                        .await
                                        .change_context(errors::ApiErrorResponse::InternalServerError)
                                        .attach_printable_lazy(|| format!(
                                            "Failed while pushing refund sync task in scheduler: refund_id: {}",
                                            refund.id.get_string_repr()
                                        ))?;
                                    Ok((updated_refund_data, raw_connector_response))
                                }
                                Err(err) => Err(err),
                            }
                        }
                    }
                }
                _ => {
                    // Sync the refund for status check
                    //[#300]: return refund status response
                    match refund_type {
                        api_models::refunds::RefundType::Scheduled => {
                            add_refund_sync_task(db, &refund, runner)
                                .await
                                .change_context(errors::ApiErrorResponse::InternalServerError)
                                .attach_printable_lazy(|| format!("Failed while pushing refund sync task in scheduler: refund_id: {}", refund.id.get_string_repr()))?;
                            Ok((refund, None))
                        }
                        api_models::refunds::RefundType::Instant => {
                            // [#255]: This is not possible in schedule_refund_execution as it will always be scheduled
                            // sync_refund_with_gateway(data, &refund).await
                            Ok((refund, None))
                        }
                    }
                }
            }
        }
        //  [#255]: This is not allowed to be otherwise or all
        _ => Ok((refund, None)),
    }?;
    Ok(result)
}

#[instrument]
pub fn refund_to_refund_core_workflow_model(
    refund: &diesel_refund::Refund,
) -> diesel_refund::RefundCoreWorkflow {
    diesel_refund::RefundCoreWorkflow {
        refund_id: refund.id.clone(),
        connector_transaction_id: refund.connector_transaction_id.clone(),
        merchant_id: refund.merchant_id.clone(),
        payment_id: refund.payment_id.clone(),
        processor_transaction_data: refund.processor_transaction_data.clone(),
    }
}

#[instrument(skip_all)]
pub async fn add_refund_execute_task(
    db: &dyn db::StorageInterface,
    refund: &diesel_refund::Refund,
    runner: storage::ProcessTrackerRunner,
) -> errors::RouterResult<storage::ProcessTracker> {
    let task = "EXECUTE_REFUND";
    let process_tracker_id = format!("{runner}_{task}_{}", refund.id.get_string_repr());
    let tag = ["REFUND"];
    let schedule_time = common_utils::date_time::now();
    let refund_workflow_tracking_data = refund_to_refund_core_workflow_model(refund);
    let process_tracker_entry = storage::ProcessTrackerNew::new(
        process_tracker_id,
        task,
        runner,
        tag,
        refund_workflow_tracking_data,
        None,
        schedule_time,
        common_types::consts::API_VERSION,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to construct refund execute process tracker task")?;

    let response = db
        .insert_process(process_tracker_entry)
        .await
        .to_duplicate_response(errors::ApiErrorResponse::DuplicateRefundRequest)
        .attach_printable_lazy(|| {
            format!(
                "Failed while inserting task in process_tracker: refund_id: {}",
                refund.id.get_string_repr()
            )
        })?;
    Ok(response)
}

#[instrument(skip_all)]
pub async fn add_refund_sync_task(
    db: &dyn db::StorageInterface,
    refund: &diesel_refund::Refund,
    runner: storage::ProcessTrackerRunner,
) -> errors::RouterResult<storage::ProcessTracker> {
    let task = "SYNC_REFUND";
    let process_tracker_id = format!("{runner}_{task}_{}", refund.id.get_string_repr());
    let schedule_time = common_utils::date_time::now();
    let refund_workflow_tracking_data = refund_to_refund_core_workflow_model(refund);
    let tag = ["REFUND"];
    let process_tracker_entry = storage::ProcessTrackerNew::new(
        process_tracker_id,
        task,
        runner,
        tag,
        refund_workflow_tracking_data,
        None,
        schedule_time,
        common_types::consts::API_VERSION,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to construct refund sync process tracker task")?;

    let response = db
        .insert_process(process_tracker_entry)
        .await
        .to_duplicate_response(errors::ApiErrorResponse::DuplicateRefundRequest)
        .attach_printable_lazy(|| {
            format!(
                "Failed while inserting task in process_tracker: refund_id: {}",
                refund.id.get_string_repr()
            )
        })?;
    metrics::TASKS_ADDED_COUNT.add(1, router_env::metric_attributes!(("flow", "Refund")));

    Ok(response)
}
