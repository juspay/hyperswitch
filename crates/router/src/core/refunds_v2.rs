use std::{fmt::Debug, str::FromStr};

use api_models::{enums::Connector, refunds::RefundErrorDetails};
use common_utils::{id_type, types as common_utils_types};
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
    merchant_context: domain::MerchantContext,
    req: refunds::RefundsCreateRequest,
) -> errors::RouterResponse<refunds::RefundResponse> {
    let db = &*state.store;
    let (payment_intent, payment_attempt, amount);

    payment_intent = db
        .find_payment_intent_by_id(
            &(&state).into(),
            &req.payment_id,
            merchant_context.get_merchant_key_store(),
            merchant_context.get_merchant_account().storage_scheme,
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
            &(&state).into(),
            merchant_context.get_merchant_key_store(),
            &req.payment_id,
            merchant_context.get_merchant_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::SuccessfulPaymentNotFound)?;

    let global_refund_id = id_type::GlobalRefundId::generate(&state.conf.cell_information.id);

    tracing::Span::current().record("global_refund_id", global_refund_id.get_string_repr());

    Box::pin(validate_and_create_refund(
        &state,
        &merchant_context,
        &payment_attempt,
        &payment_intent,
        amount,
        req,
        global_refund_id,
    ))
    .await
    .map(services::ApplicationResponse::Json)
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn trigger_refund_to_gateway(
    state: &SessionState,
    refund: &storage::Refund,
    merchant_context: &domain::MerchantContext,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: &storage::PaymentIntent,
) -> errors::RouterResult<storage::Refund> {
    let db = &*state.store;

    let mca_id = payment_attempt.get_attempt_merchant_connector_account_id()?;

    let storage_scheme = merchant_context.get_merchant_account().storage_scheme;

    let mca = db
        .find_merchant_connector_account_by_id(
            &state.into(),
            &mca_id,
            merchant_context.get_merchant_key_store(),
        )
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

    let mut router_data = core_utils::construct_refund_router_data(
        state,
        connector_enum,
        merchant_context,
        payment_intent,
        payment_attempt,
        refund,
        &mca,
    )
    .await?;

    let add_access_token_result =
        access_token::add_access_token(state, &connector, merchant_context, &router_data, None)
            .await?;

    logger::debug!(refund_router_data=?router_data);

    access_token::update_router_data_with_access_token_result(
        &add_access_token_result,
        &mut router_data,
        &payments::CallConnectorAction::Trigger,
    );

    let connector_response =
        call_connector_service(state, &connector, add_access_token_result, router_data).await;

    let refund_update = get_refund_update_object(
        state,
        &connector,
        &storage_scheme,
        merchant_context,
        &connector_response,
    )
    .await;

    let response = match refund_update {
        Some(refund_update) => state
            .store
            .update_refund(
                refund.to_owned(),
                refund_update,
                merchant_context.get_merchant_account().storage_scheme,
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
    // Implement outgoing webhooks here
    connector_response.to_refund_failed_response()?;
    Ok(response)
}

async fn call_connector_service<F>(
    state: &SessionState,
    connector: &api::ConnectorData,
    add_access_token_result: types::AddAccessTokenResult,
    router_data: RouterData<F, types::RefundsData, types::RefundsResponseData>,
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
    merchant_context: &domain::MerchantContext,
    router_data_response: &Result<
        RouterData<api::Execute, types::RefundsData, types::RefundsResponseData>,
        error_stack::Report<errors::ConnectorError>,
    >,
) -> Option<storage::RefundUpdate> {
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
                    merchant_context,
                )),
            }
        }
    }
}

fn get_connector_implementation_error_refund_update(
    error: &error_stack::Report<errors::ConnectorError>,
    storage_scheme: enums::MerchantStorageScheme,
) -> Option<storage::RefundUpdate> {
    Option::<storage::RefundUpdate>::foreign_from((error.current_context(), storage_scheme))
}

async fn get_connector_error_refund_update(
    state: &SessionState,
    err: ErrorResponse,
    connector: &api::ConnectorData,
    storage_scheme: &enums::MerchantStorageScheme,
) -> storage::RefundUpdate {
    let unified_error_object = get_unified_error_and_message(state, &err, connector).await;

    storage::RefundUpdate::build_error_update_for_unified_error_and_message(
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
    merchant_context: &domain::MerchantContext,
) -> storage::RefundUpdate {
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
                        merchant_context.get_merchant_account().get_id().clone()
                    ),
                ),
            );

            storage::RefundUpdate::build_error_update_for_integrity_check_failure(
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

            storage::RefundUpdate::build_refund_update(
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
    for Option<storage::RefundUpdate>
{
    fn foreign_from(
        (from, storage_scheme): (&errors::ConnectorError, enums::MerchantStorageScheme),
    ) -> Self {
        match from {
            errors::ConnectorError::NotImplemented(message) => {
                Some(storage::RefundUpdate::ErrorUpdate {
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
                Some(storage::RefundUpdate::ErrorUpdate {
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

// ********************************************** REFUND SYNC **********************************************

#[instrument(skip_all)]
pub async fn refund_retrieve_core_with_refund_id(
    state: SessionState,
    merchant_context: domain::MerchantContext,
    profile: domain::Profile,
    request: refunds::RefundsRetrieveRequest,
) -> errors::RouterResponse<refunds::RefundResponse> {
    let refund_id = request.refund_id.clone();
    let db = &*state.store;
    let profile_id = profile.get_id().to_owned();
    let refund = db
        .find_refund_by_id(
            &refund_id,
            merchant_context.get_merchant_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::RefundNotFound)?;

    let response = Box::pin(refund_retrieve_core(
        state.clone(),
        merchant_context,
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
    merchant_context: domain::MerchantContext,
    profile_id: Option<id_type::ProfileId>,
    request: refunds::RefundsRetrieveRequest,
    refund: storage::Refund,
) -> errors::RouterResult<storage::Refund> {
    let db = &*state.store;

    let key_manager_state = &(&state).into();
    core_utils::validate_profile_id_from_auth_layer(profile_id, &refund)?;
    let payment_id = &refund.payment_id;
    let payment_intent = db
        .find_payment_intent_by_id(
            key_manager_state,
            payment_id,
            merchant_context.get_merchant_key_store(),
            merchant_context.get_merchant_account().storage_scheme,
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
            key_manager_state,
            merchant_context.get_merchant_key_store(),
            &active_attempt_id,
            merchant_context.get_merchant_account().storage_scheme,
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

    let refund = storage::Refund {
        unified_message: unified_translated_message,
        ..refund
    };

    let response = if should_call_refund(&refund, request.force_sync.unwrap_or(false)) {
        Box::pin(sync_refund_with_gateway(
            &state,
            &merchant_context,
            &payment_attempt,
            &payment_intent,
            &refund,
        ))
        .await
    } else {
        Ok(refund)
    }?;
    Ok(response)
}

fn should_call_refund(refund: &diesel_models::refund::Refund, force_sync: bool) -> bool {
    // This implies, we cannot perform a refund sync & `the connector_refund_id`
    // doesn't exist
    let predicate1 = refund.connector_refund_id.is_some();

    // This allows refund sync at connector level if force_sync is enabled, or
    // checks if the refund has failed
    let predicate2 = force_sync
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
    merchant_context: &domain::MerchantContext,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: &storage::PaymentIntent,
    refund: &storage::Refund,
) -> errors::RouterResult<storage::Refund> {
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
        .find_merchant_connector_account_by_id(
            &state.into(),
            &mca_id,
            merchant_context.get_merchant_key_store(),
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch merchant connector account")?;

    let connector_enum = mca.connector_name;

    let mut router_data = core_utils::construct_refund_router_data::<api::RSync>(
        state,
        connector_enum,
        merchant_context,
        payment_intent,
        payment_attempt,
        refund,
        &mca,
    )
    .await?;

    let add_access_token_result =
        access_token::add_access_token(state, &connector, merchant_context, &router_data, None)
            .await?;

    logger::debug!(refund_retrieve_router_data=?router_data);

    access_token::update_router_data_with_access_token_result(
        &add_access_token_result,
        &mut router_data,
        &payments::CallConnectorAction::Trigger,
    );

    let connector_response =
        call_connector_service(state, &connector, add_access_token_result, router_data)
            .await
            .to_refund_failed_response()?;

    let connector_response = perform_integrity_check(connector_response);

    let refund_update =
        build_refund_update_for_rsync(&connector, merchant_context, connector_response);

    let response = state
        .store
        .update_refund(
            refund.to_owned(),
            refund_update,
            merchant_context.get_merchant_account().storage_scheme,
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
    Ok(response)
}

pub fn build_refund_update_for_rsync(
    connector: &api::ConnectorData,
    merchant_context: &domain::MerchantContext,
    router_data_response: RouterData<api::RSync, types::RefundsData, types::RefundsResponseData>,
) -> storage::RefundUpdate {
    let merchant_account = merchant_context.get_merchant_account();
    let storage_scheme = &merchant_context.get_merchant_account().storage_scheme;

    match router_data_response.response {
        Err(error_message) => {
            let refund_status = match error_message.status_code {
                // marking failure for 2xx because this is genuine refund failure
                200..=299 => Some(enums::RefundStatus::Failure),
                _ => None,
            };
            let refund_error_message = error_message.reason.or(Some(error_message.message));
            let refund_error_code = Some(error_message.code);

            storage::RefundUpdate::build_error_update_for_refund_failure(
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

                storage::RefundUpdate::build_error_update_for_integrity_check_failure(
                    err.field_names,
                    connector_refund_id,
                    storage_scheme,
                )
            }
            Ok(()) => {
                let connector_refund_id =
                    common_utils_types::ConnectorTransactionId::from(response.connector_refund_id);

                storage::RefundUpdate::build_refund_update(
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
    merchant_context: &domain::MerchantContext,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: &storage::PaymentIntent,
    refund_amount: common_utils_types::MinorUnit,
    req: refunds::RefundsCreateRequest,
    global_refund_id: id_type::GlobalRefundId,
) -> errors::RouterResult<refunds::RefundResponse> {
    let db = &*state.store;

    let refund_type = req.refund_type.unwrap_or_default();

    let merchant_reference_id = req.merchant_reference_id;

    let predicate = req
        .merchant_id
        .as_ref()
        .map(|merchant_id| merchant_id != merchant_context.get_merchant_account().get_id());

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
            merchant_context.get_merchant_account().get_id(),
            &connector_payment_id,
            merchant_context.get_merchant_account().storage_scheme,
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
    let refund_create_req = storage::RefundNew {
        id: global_refund_id,
        merchant_reference_id: merchant_reference_id.clone(),
        external_reference_id: Some(merchant_reference_id.get_string_repr().to_string()),
        payment_id: req.payment_id,
        merchant_id: merchant_context.get_merchant_account().get_id().clone(),
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
        organization_id: merchant_context
            .get_merchant_account()
            .organization_id
            .clone(),
        processor_transaction_data,
        processor_refund_data: None,
    };

    let refund = match db
        .insert_refund(
            refund_create_req,
            merchant_context.get_merchant_account().storage_scheme,
        )
        .await
    {
        Ok(refund) => {
            Box::pin(schedule_refund_execution(
                state,
                refund.clone(),
                refund_type,
                merchant_context,
                payment_attempt,
                payment_intent,
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

    let refund = storage::Refund {
        unified_message: unified_translated_message,
        ..refund
    };

    api::RefundResponse::foreign_try_from(refund)
}

impl ForeignTryFrom<storage::Refund> for api::RefundResponse {
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn foreign_try_from(refund: storage::Refund) -> Result<Self, Self::Error> {
        let refund = refund;

        let profile_id = refund
            .profile_id
            .clone()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Profile id not found")?;

        let merchant_connector_id = refund
            .connector_id
            .clone()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Connector id not found")?;

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
            merchant_connector_id,
            merchant_reference_id: Some(refund.merchant_reference_id),
            error_details: Some(RefundErrorDetails {
                code: refund.refund_error_code.unwrap_or_default(),
                message: refund.refund_error_message.unwrap_or_default(),
            }),
            connector_refund_reference_id: None,
        })
    }
}

// ********************************************** PROCESS TRACKER **********************************************

#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn schedule_refund_execution(
    state: &SessionState,
    refund: storage::Refund,
    refund_type: api_models::refunds::RefundType,
    merchant_context: &domain::MerchantContext,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: &storage::PaymentIntent,
) -> errors::RouterResult<storage::Refund> {
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

                            Ok(refund)
                        }
                        api_models::refunds::RefundType::Instant => {
                            let update_refund = Box::pin(trigger_refund_to_gateway(
                                state,
                                &refund,
                                merchant_context,
                                payment_attempt,
                                payment_intent,
                            ))
                            .await;

                            match update_refund {
                                Ok(updated_refund_data) => {
                                    add_refund_sync_task(db, &updated_refund_data, runner)
                                        .await
                                        .change_context(errors::ApiErrorResponse::InternalServerError)
                                        .attach_printable_lazy(|| format!(
                                            "Failed while pushing refund sync task in scheduler: refund_id: {}",
                                            refund.id.get_string_repr()
                                        ))?;
                                    Ok(updated_refund_data)
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
                            Ok(refund)
                        }
                        api_models::refunds::RefundType::Instant => {
                            // [#255]: This is not possible in schedule_refund_execution as it will always be scheduled
                            // sync_refund_with_gateway(data, &refund).await
                            Ok(refund)
                        }
                    }
                }
            }
        }
        //  [#255]: This is not allowed to be otherwise or all
        _ => Ok(refund),
    }?;
    Ok(result)
}

#[instrument]
pub fn refund_to_refund_core_workflow_model(
    refund: &storage::Refund,
) -> storage::RefundCoreWorkflow {
    storage::RefundCoreWorkflow {
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
    refund: &storage::Refund,
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
    refund: &storage::Refund,
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
