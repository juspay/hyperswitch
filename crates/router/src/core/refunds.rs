#[cfg(feature = "olap")]
use std::collections::HashMap;

#[cfg(feature = "olap")]
use api_models::admin::MerchantConnectorInfo;
use common_enums::ExecutionMode;
use common_utils::{
    ext_traits::{AsyncExt, StringExt},
    types::{ConnectorTransactionId, MinorUnit},
};
use diesel_models::{process_tracker::business_status, refund as diesel_refund};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    router_data::ErrorResponse, router_request_types::SplitRefundsRequest,
};
use hyperswitch_interfaces::integrity::{CheckIntegrity, FlowIntegrity, GetIntegrityObject};
use router_env::{instrument, tracing};
use scheduler::{
    consumer::types::process_data, errors as sch_errors, utils as process_tracker_utils,
};
#[cfg(feature = "olap")]
use strum::IntoEnumIterator;

use crate::{
    consts,
    core::{
        errors::{self, ConnectorErrorExt, RouterResponse, RouterResult, StorageErrorExt},
        payments::{self, access_token, helpers, helpers::MerchantConnectorAccountType},
        refunds::transformers::SplitRefundInput,
        unified_connector_service,
        utils::{
            self as core_utils, refunds_transformers as transformers,
            refunds_validator as validator,
        },
    },
    db, logger,
    routes::{metrics, SessionState},
    services,
    types::{
        self,
        api::{self, refunds},
        domain,
        storage::{self, enums},
        transformers::{ForeignFrom, ForeignInto},
    },
    utils::{self, OptionExt},
};

// ********************************************** REFUND EXECUTE **********************************************

#[instrument(skip_all)]
pub async fn refund_create_core(
    state: SessionState,
    platform: domain::Platform,
    _profile_id: Option<common_utils::id_type::ProfileId>,
    req: refunds::RefundRequest,
) -> RouterResponse<refunds::RefundResponse> {
    let db = &*state.store;
    let (merchant_id, payment_intent, payment_attempt, amount);

    merchant_id = platform.get_processor().get_account().get_id();

    payment_intent = db
        .find_payment_intent_by_payment_id_merchant_id(
            &req.payment_id,
            merchant_id,
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

    // Amount is not passed in request refer from payment intent.
    amount = req
        .amount
        .or(payment_intent.amount_captured)
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("amount captured is none in a successful payment")?;

    //[#299]: Can we change the flow based on some workflow idea
    utils::when(amount <= MinorUnit::new(0), || {
        Err(report!(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "amount".to_string(),
            expected_format: "positive integer".to_string()
        })
        .attach_printable("amount less than or equal to zero"))
    })?;

    payment_attempt = db
        .find_payment_attempt_last_successful_or_partially_captured_attempt_by_payment_id_merchant_id(
            &req.payment_id,
            merchant_id,
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::SuccessfulPaymentNotFound)?;

    let creds_identifier = req
        .merchant_connector_details
        .as_ref()
        .map(|mcd| mcd.creds_identifier.to_owned());
    req.merchant_connector_details
        .to_owned()
        .async_map(|mcd| async {
            helpers::insert_merchant_connector_creds_to_config(db, merchant_id, mcd).await
        })
        .await
        .transpose()?;

    Box::pin(validate_and_create_refund(
        &state,
        &platform,
        &payment_attempt,
        &payment_intent,
        amount,
        req,
        creds_identifier,
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
    creds_identifier: Option<String>,
    split_refunds: Option<SplitRefundsRequest>,
    all_keys_required: Option<bool>,
) -> RouterResult<(diesel_refund::Refund, Option<masking::Secret<String>>)> {
    let routed_through = payment_attempt
        .connector
        .clone()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to retrieve connector from payment attempt")?;

    let storage_scheme = platform.get_processor().get_account().storage_scheme;
    metrics::REFUND_COUNT.add(
        1,
        router_env::metric_attributes!(("connector", routed_through.clone())),
    );

    let connector: api::ConnectorData = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &routed_through,
        api::GetToken::Connector,
        payment_attempt.merchant_connector_id.clone(),
    )?;

    let currency = payment_attempt.currency.ok_or_else(|| {
        report!(errors::ApiErrorResponse::InternalServerError).attach_printable(
            "Transaction in invalid. Missing field \"currency\" in payment_attempt.",
        )
    })?;

    validator::validate_for_valid_refunds(payment_attempt, connector.connector_name)?;

    // Fetch merchant connector account
    let profile_id = payment_intent
        .profile_id
        .as_ref()
        .get_required_value("profile_id")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("profile_id is not set in payment_intent")?;

    let merchant_connector_account = helpers::get_merchant_connector_account(
        state,
        platform.get_processor().get_account().get_id(),
        creds_identifier.as_deref(),
        platform.get_processor().get_key_store(),
        profile_id,
        &routed_through,
        payment_attempt.merchant_connector_id.as_ref(),
    )
    .await?;

    let mut router_data = core_utils::construct_refund_router_data(
        state,
        &routed_through,
        platform,
        (payment_attempt.get_total_amount(), currency),
        payment_intent,
        payment_attempt,
        refund,
        split_refunds,
        &merchant_connector_account,
    )
    .await?;

    // Add access token for both UCS and direct connector paths
    let add_access_token_result = Box::pin(access_token::add_access_token(
        state,
        &connector,
        &router_data,
        creds_identifier.as_deref(),
    ))
    .await?;

    logger::debug!(refund_router_data=?router_data);

    access_token::update_router_data_with_access_token_result(
        &add_access_token_result,
        &mut router_data,
        &payments::CallConnectorAction::Trigger,
    );

    // Common access token handling for all flows
    let router_data_res = if !(add_access_token_result.connector_supports_access_token
        && router_data.access_token.is_none())
    {
        // Access token available or not needed - proceed with execution

        // Check which gateway system to use for refunds
        let (execution_path, updated_state) =
            unified_connector_service::should_call_unified_connector_service(
                state,
                platform,
                &router_data,
                None::<&payments::PaymentData<api::Execute>>, // No payment data for refunds
                payments::CallConnectorAction::Trigger,
                None,
            )
            .await?;

        router_env::logger::info!(
            refund_id = refund.refund_id,
            execution_path = ?execution_path,
            "Executing refund via {execution_path:?}"
        );

        // Execute refund based on gateway system decision
        match execution_path {
            common_enums::ExecutionPath::UnifiedConnectorService => {
                unified_connector_service::call_unified_connector_service_for_refund_execute(
                    state,
                    platform,
                    router_data.clone(),
                    ExecutionMode::Primary,
                    merchant_connector_account,
                )
                .await
                .attach_printable(format!(
                    "UCS refund execution failed for connector: {}, refund_id: {}",
                    connector.connector_name, router_data.request.refund_id
                ))?
            }
            common_enums::ExecutionPath::Direct => {
                Box::pin(execute_refund_execute_via_direct(
                    state,
                    &connector,
                    platform,
                    refund,
                    router_data,
                    all_keys_required,
                ))
                .await?
            }
            common_enums::ExecutionPath::ShadowUnifiedConnectorService => {
                Box::pin(execute_refund_execute_via_direct_with_ucs_shadow(
                    &updated_state,
                    &connector,
                    platform,
                    refund,
                    router_data,
                    merchant_connector_account.clone(),
                    all_keys_required,
                ))
                .await?
            }
        }
    } else {
        // Access token needed but not available - return error router_data
        router_data
    };

    let refund_update = match router_data_res.response {
        Err(err) => {
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

            let (unified_code, unified_message) = if let Some((code, message)) =
                gsm_unified_code.as_ref().zip(gsm_unified_message.as_ref())
            {
                (code.to_owned(), message.to_owned())
            } else {
                (
                    consts::DEFAULT_UNIFIED_ERROR_CODE.to_owned(),
                    consts::DEFAULT_UNIFIED_ERROR_MESSAGE.to_owned(),
                )
            };

            diesel_refund::RefundUpdate::ErrorUpdate {
                refund_status: Some(enums::RefundStatus::Failure),
                refund_error_message: err.reason.or(Some(err.message)),
                refund_error_code: Some(err.code),
                updated_by: storage_scheme.to_string(),
                connector_refund_id: None,
                processor_refund_data: None,
                unified_code: Some(unified_code),
                unified_message: Some(unified_message),
                issuer_error_code: err.network_decline_code,
                issuer_error_message: err.network_error_message,
            }
        }
        Ok(response) => {
            // match on connector integrity checks
            match router_data_res.integrity_check.clone() {
                Err(err) => {
                    let (refund_connector_transaction_id, processor_refund_data) =
                        err.connector_transaction_id.map_or((None, None), |txn_id| {
                            let (refund_id, refund_data) =
                                ConnectorTransactionId::form_id_and_data(txn_id);
                            (Some(refund_id), refund_data)
                        });
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
                    diesel_refund::RefundUpdate::ErrorUpdate {
                        refund_status: Some(enums::RefundStatus::ManualReview),
                        refund_error_message: Some(format!(
                            "Integrity Check Failed! as data mismatched for fields {}",
                            err.field_names
                        )),
                        refund_error_code: Some("IE".to_string()),
                        updated_by: storage_scheme.to_string(),
                        connector_refund_id: refund_connector_transaction_id,
                        processor_refund_data,
                        unified_code: None,
                        unified_message: None,
                        issuer_error_code: None,
                        issuer_error_message: None,
                    }
                }
                Ok(()) => {
                    if response.refund_status == diesel_models::enums::RefundStatus::Success {
                        metrics::SUCCESSFUL_REFUND.add(
                            1,
                            router_env::metric_attributes!((
                                "connector",
                                connector.connector_name.to_string(),
                            )),
                        )
                    }
                    let (connector_refund_id, processor_refund_data) =
                        ConnectorTransactionId::form_id_and_data(response.connector_refund_id);
                    diesel_refund::RefundUpdate::Update {
                        connector_refund_id,
                        refund_status: response.refund_status,
                        sent_to_gateway: true,
                        refund_error_message: None,
                        refund_arn: "".to_string(),
                        updated_by: storage_scheme.to_string(),
                        processor_refund_data,
                    }
                }
            }
        }
    };

    let response = state
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
                refund.refund_id
            )
        })?;
    utils::trigger_refund_outgoing_webhook(
        state,
        platform,
        &response,
        payment_attempt.profile_id.clone(),
    )
    .await
    .map_err(|error| logger::warn!(refunds_outgoing_webhook_error=?error))
    .ok();
    Ok((response, router_data_res.raw_connector_response))
}

// ============================================================================
// REFUND EXECUTION FUNCTIONS
// ============================================================================

/// Execute refund via Direct connector
async fn execute_refund_execute_via_direct(
    state: &SessionState,
    connector: &api::ConnectorData,
    platform: &domain::Platform,
    refund: &diesel_refund::Refund,
    router_data: types::RefundExecuteRouterData,
    all_keys_required: Option<bool>,
) -> RouterResult<types::RefundExecuteRouterData> {
    let storage_scheme = platform.get_processor().get_account().storage_scheme;

    let connector_integration: services::BoxedRefundConnectorIntegrationInterface<
        api::Execute,
        types::RefundsData,
        types::RefundsResponseData,
    > = connector.connector.get_connector_integration();

    let router_data_res = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
        all_keys_required,
    )
    .await;

    // Handle specific connector errors and update refund status if needed
    let option_refund_error_update =
        router_data_res
            .as_ref()
            .err()
            .and_then(|error| match error.current_context() {
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
                        issuer_error_code: None,
                        issuer_error_message: None,
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
                        issuer_error_code: None,
                        issuer_error_message: None,
                    })
                }
                _ => None,
            });

    // Update the refund status as failure if connector_error is NotImplemented or NotSupported
    if let Some(refund_error_update) = option_refund_error_update {
        state
            .store
            .update_refund(refund.to_owned(), refund_error_update, storage_scheme)
            .await
            .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
            .attach_printable_lazy(|| {
                format!(
                    "Failed while updating refund: refund_id: {}",
                    refund.refund_id
                )
            })?;
    }

    let mut refund_router_data_res = router_data_res.to_refund_failed_response()?;

    // Perform integrity check
    let integrity_result = check_refund_integrity(
        &refund_router_data_res.request,
        &refund_router_data_res.response,
    );
    refund_router_data_res.integrity_check = integrity_result;

    Ok(refund_router_data_res)
}

/// Execute refund via Direct connector with UCS shadow comparison
async fn execute_refund_execute_via_direct_with_ucs_shadow(
    state: &SessionState,
    connector: &api::ConnectorData,
    platform: &domain::Platform,
    refund: &diesel_refund::Refund,
    router_data: types::RefundExecuteRouterData,
    merchant_connector_account: MerchantConnectorAccountType,
    all_keys_required: Option<bool>,
) -> RouterResult<types::RefundExecuteRouterData> {
    // Execute Direct connector (PRIMARY)
    let direct_result = Box::pin(execute_refund_execute_via_direct(
        state,
        connector,
        platform,
        refund,
        router_data.clone(),
        all_keys_required,
    ))
    .await;

    // Execute UCS in parallel (SHADOW - for comparison only)
    let ucs_router_data = router_data.clone();
    let ucs_platform = platform.clone();
    let ucs_state = state.clone();

    // Clone direct result for comparison (if successful)
    let direct_router_data_for_comparison = direct_result.as_ref().ok().cloned();

    tokio::spawn(async move {
        let ucs_result =
            unified_connector_service::call_unified_connector_service_for_refund_execute(
                &ucs_state,
                &ucs_platform,
                ucs_router_data,
                ExecutionMode::Shadow,
                merchant_connector_account,
            )
            .await;

        match (ucs_result, direct_router_data_for_comparison) {
            (Ok(ucs_router_data), Some(direct_router_data)) => {
                unified_connector_service::serialize_router_data_and_send_to_comparison_service(
                    &ucs_state,
                    direct_router_data,
                    ucs_router_data,
                )
                .await
                .inspect_err(|e| {
                    router_env::logger::debug!(
                        "Shadow UCS refund execute comparison failed: {:?}",
                        e
                    );
                })
                .ok();
            }
            (Err(e), _) => {
                router_env::logger::debug!(
                    "Skipping refund execute comparison - UCS shadow execute failed: {:?}",
                    e
                );
            }
            (_, None) => {
                router_env::logger::debug!(
                    "Skipping refund execute comparison - direct execute failed"
                );
            }
        }
    });

    // Return PRIMARY result (Direct connector response)
    direct_result
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

pub async fn refund_response_wrapper<F, Fut, T, Req>(
    state: SessionState,
    platform: domain::Platform,
    profile_id: Option<common_utils::id_type::ProfileId>,
    request: Req,
    f: F,
) -> RouterResponse<refunds::RefundResponse>
where
    F: Fn(SessionState, domain::Platform, Option<common_utils::id_type::ProfileId>, Req) -> Fut,
    Fut: futures::Future<Output = RouterResult<T>>,
    T: ForeignInto<refunds::RefundResponse>,
{
    Ok(services::ApplicationResponse::Json(
        f(state, platform, profile_id, request)
            .await?
            .foreign_into(),
    ))
}

#[instrument(skip_all)]
pub async fn refund_retrieve_core(
    state: SessionState,
    platform: domain::Platform,
    profile_id: Option<common_utils::id_type::ProfileId>,
    request: refunds::RefundsRetrieveRequest,
    refund: diesel_refund::Refund,
) -> RouterResult<(diesel_refund::Refund, Option<masking::Secret<String>>)> {
    let db = &*state.store;
    let merchant_id = platform.get_processor().get_account().get_id();
    core_utils::validate_profile_id_from_auth_layer(profile_id, &refund)?;
    let payment_id = &refund.payment_id;
    let payment_intent = db
        .find_payment_intent_by_payment_id_merchant_id(
            payment_id,
            merchant_id,
            platform.get_processor().get_key_store(),
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    let payment_attempt = db
        .find_payment_attempt_by_connector_transaction_id_payment_id_merchant_id(
            &refund.connector_transaction_id,
            payment_id,
            merchant_id,
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::InternalServerError)?;

    let creds_identifier = request
        .merchant_connector_details
        .as_ref()
        .map(|mcd| mcd.creds_identifier.to_owned());
    request
        .merchant_connector_details
        .to_owned()
        .async_map(|mcd| async {
            helpers::insert_merchant_connector_creds_to_config(db, merchant_id, mcd).await
        })
        .await
        .transpose()?;

    let split_refunds_req = core_utils::get_split_refunds(SplitRefundInput {
        split_payment_request: payment_intent.split_payments.clone(),
        payment_charges: payment_attempt.charges.clone(),
        charge_id: payment_attempt.charge_id.clone(),
        refund_request: refund.split_refunds.clone(),
    })?;

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
        request.all_keys_required.unwrap_or(false),
    ) {
        Box::pin(sync_refund_with_gateway(
            &state,
            &platform,
            &payment_attempt,
            &payment_intent,
            &refund,
            creds_identifier,
            split_refunds_req,
            request.all_keys_required,
        ))
        .await
    } else {
        Ok((refund, None))
    }?;

    Ok((response, raw_connector_response))
}

fn should_call_refund(
    refund: &diesel_models::refund::Refund,
    force_sync: bool,
    all_keys_required: bool,
) -> bool {
    // This implies, we cannot perform a refund sync & `the connector_refund_id`
    // doesn't exist
    let predicate1 = refund.connector_refund_id.is_some();

    // This allows refund sync at connector level if all_keys_required or force_sync is enabled, or
    // checks if the refund has failed
    let predicate2 = all_keys_required
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
    creds_identifier: Option<String>,
    split_refunds: Option<SplitRefundsRequest>,
    all_keys_required: Option<bool>,
) -> RouterResult<(diesel_refund::Refund, Option<masking::Secret<String>>)> {
    let connector_id = refund.connector.to_string();
    let connector: api::ConnectorData = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_id,
        api::GetToken::Connector,
        payment_attempt.merchant_connector_id.clone(),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get the connector")?;

    let storage_scheme = platform.get_processor().get_account().storage_scheme;

    let currency = payment_attempt.currency.get_required_value("currency")?;

    // Fetch merchant connector account
    let profile_id = payment_intent
        .profile_id
        .as_ref()
        .get_required_value("profile_id")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("profile_id is not set in payment_intent")?;

    let merchant_connector_account = helpers::get_merchant_connector_account(
        state,
        platform.get_processor().get_account().get_id(),
        creds_identifier.as_deref(),
        platform.get_processor().get_key_store(),
        profile_id,
        &connector_id,
        payment_attempt.merchant_connector_id.as_ref(),
    )
    .await?;

    let mut router_data = core_utils::construct_refund_router_data::<api::RSync>(
        state,
        &connector_id,
        platform,
        (payment_attempt.get_total_amount(), currency),
        payment_intent,
        payment_attempt,
        refund,
        split_refunds,
        &merchant_connector_account,
    )
    .await?;

    // Add access token for both UCS and direct connector paths
    let add_access_token_result = Box::pin(access_token::add_access_token(
        state,
        &connector,
        &router_data,
        creds_identifier.as_deref(),
    ))
    .await?;

    logger::debug!(refund_retrieve_router_data=?router_data);

    access_token::update_router_data_with_access_token_result(
        &add_access_token_result,
        &mut router_data,
        &payments::CallConnectorAction::Trigger,
    );

    // Common access token handling for all flows
    let router_data_res = if !(add_access_token_result.connector_supports_access_token
        && router_data.access_token.is_none())
    {
        // Access token available or not needed - proceed with execution

        // Check which gateway system to use for refund sync
        let (execution_path, updated_state) =
            unified_connector_service::should_call_unified_connector_service(
                state,
                platform,
                &router_data,
                None::<&payments::PaymentData<api::RSync>>, // No payment data for refunds
                payments::CallConnectorAction::Trigger,
                None,
            )
            .await?;

        router_env::logger::info!(
            refund_id = router_data.request.refund_id,
            execution_path = ?execution_path,
            "Executing refund sync via {execution_path:?}"
        );

        // Execute refund sync based on gateway system decision
        match execution_path {
            common_enums::ExecutionPath::UnifiedConnectorService => {
                unified_connector_service::call_unified_connector_service_for_refund_sync(
                    state,
                    platform,
                    router_data.clone(),
                    ExecutionMode::Primary,
                    merchant_connector_account,
                )
                .await
                .attach_printable(format!(
                    "UCS refund sync failed for connector: {}, refund_id: {}",
                    connector.connector_name, router_data.request.refund_id
                ))?
            }
            common_enums::ExecutionPath::Direct => {
                Box::pin(execute_refund_sync_via_direct(
                    state,
                    &connector,
                    router_data,
                    all_keys_required,
                ))
                .await?
            }
            common_enums::ExecutionPath::ShadowUnifiedConnectorService => {
                Box::pin(execute_refund_sync_via_direct_with_ucs_shadow(
                    &updated_state,
                    &connector,
                    platform,
                    router_data,
                    merchant_connector_account,
                ))
                .await?
            }
        }
    } else {
        // Access token needed but not available - return error router_data
        router_data
    };

    let refund_update = match router_data_res.response {
        Err(error_message) => {
            let refund_status = match error_message.status_code {
                // marking failure for 2xx because this is genuine refund failure
                200..=299 => Some(enums::RefundStatus::Failure),
                _ => None,
            };
            diesel_refund::RefundUpdate::ErrorUpdate {
                refund_status,
                refund_error_message: error_message.reason.or(Some(error_message.message)),
                refund_error_code: Some(error_message.code),
                updated_by: storage_scheme.to_string(),
                connector_refund_id: None,
                processor_refund_data: None,
                unified_code: None,
                unified_message: None,
                issuer_error_code: error_message.network_decline_code,
                issuer_error_message: error_message.network_error_message,
            }
        }
        Ok(response) => match router_data_res.integrity_check.clone() {
            Err(err) => {
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
                let (refund_connector_transaction_id, processor_refund_data) = err
                    .connector_transaction_id
                    .map_or((None, None), |refund_id| {
                        let (refund_id, refund_data) =
                            ConnectorTransactionId::form_id_and_data(refund_id);
                        (Some(refund_id), refund_data)
                    });
                diesel_refund::RefundUpdate::ErrorUpdate {
                    refund_status: Some(enums::RefundStatus::ManualReview),
                    refund_error_message: Some(format!(
                        "Integrity Check Failed! as data mismatched for fields {}",
                        err.field_names
                    )),
                    refund_error_code: Some("IE".to_string()),
                    updated_by: storage_scheme.to_string(),
                    connector_refund_id: refund_connector_transaction_id,
                    processor_refund_data,
                    unified_code: None,
                    unified_message: None,
                    issuer_error_code: None,
                    issuer_error_message: None,
                }
            }
            Ok(()) => {
                let (connector_refund_id, processor_refund_data) =
                    ConnectorTransactionId::form_id_and_data(response.connector_refund_id);
                diesel_refund::RefundUpdate::Update {
                    connector_refund_id,
                    refund_status: response.refund_status,
                    sent_to_gateway: true,
                    refund_error_message: None,
                    refund_arn: "".to_string(),
                    updated_by: storage_scheme.to_string(),
                    processor_refund_data,
                }
            }
        },
    };

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
                refund.refund_id
            )
        })?;
    utils::trigger_refund_outgoing_webhook(
        state,
        platform,
        &response,
        payment_attempt.profile_id.clone(),
    )
    .await
    .map_err(|error| logger::warn!(refunds_outgoing_webhook_error=?error))
    .ok();
    Ok((response, router_data_res.raw_connector_response))
}

// ============================================================================
// REFUND SYNC EXECUTION FUNCTIONS
// ============================================================================

/// Execute refund sync via Direct connector
async fn execute_refund_sync_via_direct(
    state: &SessionState,
    connector: &api::ConnectorData,
    router_data: types::RefundSyncRouterData,
    all_keys_required: Option<bool>,
) -> RouterResult<types::RefundSyncRouterData> {
    let connector_integration: services::BoxedRefundConnectorIntegrationInterface<
        api::RSync,
        types::RefundsData,
        types::RefundsResponseData,
    > = connector.connector.get_connector_integration();

    let router_data_res = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
        all_keys_required,
    )
    .await;

    let mut refund_router_data_res = router_data_res.to_refund_failed_response()?;

    // Perform integrity check
    let integrity_result = check_refund_integrity(
        &refund_router_data_res.request,
        &refund_router_data_res.response,
    );
    refund_router_data_res.integrity_check = integrity_result;

    router_env::logger::info!(
        connector = connector.connector_name.to_string(),
        refund_id = refund_router_data_res.request.refund_id,
        "Direct refund sync completed successfully"
    );

    Ok(refund_router_data_res)
}

/// Execute refund sync via Direct connector with UCS shadow comparison
async fn execute_refund_sync_via_direct_with_ucs_shadow(
    state: &SessionState,
    connector: &api::ConnectorData,
    platform: &domain::Platform,
    router_data: types::RefundSyncRouterData,
    merchant_connector_account: MerchantConnectorAccountType,
) -> RouterResult<types::RefundSyncRouterData> {
    // Execute Direct connector (PRIMARY)
    let direct_result = Box::pin(execute_refund_sync_via_direct(
        state,
        connector,
        router_data.clone(),
        None,
    ))
    .await;

    // Execute UCS in shadow mode for comparison
    let state = state.clone();
    let platform = platform.clone();
    let merchant_connector_account = merchant_connector_account.clone();
    let direct_router_data_for_comparison = direct_result.as_ref().ok().cloned();

    tokio::spawn(async move {
        let ucs_result = unified_connector_service::call_unified_connector_service_for_refund_sync(
            &state,
            &platform,
            router_data,
            ExecutionMode::Shadow,
            merchant_connector_account,
        )
        .await;

        match (ucs_result, direct_router_data_for_comparison) {
            (Ok(ucs_router_data), Some(direct_router_data)) => {
                unified_connector_service::serialize_router_data_and_send_to_comparison_service(
                    &state,
                    direct_router_data,
                    ucs_router_data,
                )
                .await
                .inspect_err(|e| {
                    router_env::logger::debug!("Shadow UCS sync comparison failed: {:?}", e);
                })
                .ok();
            }
            (Err(_), _) => {
                router_env::logger::debug!(
                    "Skipping refund sync comparison - UCS shadow sync failed"
                );
            }
            (_, None) => {
                router_env::logger::debug!("Skipping refund sync comparison - direct sync failed");
            }
        }
    });

    direct_result
}

// ********************************************** REFUND UPDATE **********************************************

pub async fn refund_update_core(
    state: SessionState,
    platform: domain::Platform,
    req: refunds::RefundUpdateRequest,
) -> RouterResponse<refunds::RefundResponse> {
    let db = state.store.as_ref();
    let refund = db
        .find_refund_by_merchant_id_refund_id(
            platform.get_processor().get_account().get_id(),
            &req.refund_id,
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::RefundNotFound)?;

    let response = db
        .update_refund(
            refund,
            diesel_refund::RefundUpdate::MetadataAndReasonUpdate {
                metadata: req.metadata,
                reason: req.reason,
                updated_by: platform
                    .get_processor()
                    .get_account()
                    .storage_scheme
                    .to_string(),
            },
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!("Unable to update refund with refund_id: {}", req.refund_id)
        })?;

    Ok(services::ApplicationResponse::Json(response.foreign_into()))
}

// ********************************************** VALIDATIONS **********************************************

#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn validate_and_create_refund(
    state: &SessionState,
    platform: &domain::Platform,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: &storage::PaymentIntent,
    refund_amount: MinorUnit,
    req: refunds::RefundRequest,
    creds_identifier: Option<String>,
) -> RouterResult<refunds::RefundResponse> {
    let db = &*state.store;
    let split_refunds = core_utils::get_split_refunds(SplitRefundInput {
        split_payment_request: payment_intent.split_payments.clone(),
        payment_charges: payment_attempt.charges.clone(),
        charge_id: payment_attempt.charge_id.clone(),
        refund_request: req.split_refunds.clone(),
    })?;

    // Only for initial dev and testing
    let refund_type = req.refund_type.unwrap_or_default();

    // If Refund Id not passed in request Generate one.

    let refund_id = core_utils::get_or_generate_id("refund_id", &req.refund_id, "ref")?;

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

    let connector_transaction_id = payment_attempt.clone().connector_transaction_id.ok_or_else(|| {
        report!(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Transaction in invalid. Missing field \"connector_transaction_id\" in payment_attempt.")
    })?;

    let all_refunds = db
        .find_refund_by_merchant_id_connector_transaction_id(
            platform.get_processor().get_account().get_id(),
            &connector_transaction_id,
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::RefundNotFound)?;

    let currency = payment_attempt.currency.get_required_value("currency")?;

    //[#249]: Add Connector Based Validation here.
    validator::validate_payment_order_age(&payment_intent.created_at, state.conf.refund.max_age)
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

    validator::validate_refund_amount(
        total_amount_captured.get_amount_as_i64(),
        &all_refunds,
        refund_amount.get_amount_as_i64(),
    )
    .change_context(errors::ApiErrorResponse::RefundAmountExceedsPaymentAmount)?;

    validator::validate_maximum_refund_against_payment_attempt(
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
        ConnectorTransactionId::form_id_and_data(connector_transaction_id);
    let refund_create_req = diesel_refund::RefundNew {
        refund_id: refund_id.to_string(),
        internal_reference_id: utils::generate_id(consts::ID_LENGTH, "refid"),
        external_reference_id: Some(refund_id.clone()),
        payment_id: req.payment_id,
        merchant_id: platform.get_processor().get_account().get_id().clone(),
        connector_transaction_id,
        connector,
        refund_type: req.refund_type.unwrap_or_default().foreign_into(),
        total_amount: payment_attempt.get_total_amount(),
        refund_amount,
        currency,
        created_at: common_utils::date_time::now(),
        modified_at: common_utils::date_time::now(),
        refund_status: enums::RefundStatus::Pending,
        metadata: req.metadata,
        description: req.reason.clone(),
        attempt_id: payment_attempt.attempt_id.clone(),
        refund_reason: req.reason,
        profile_id: payment_intent.profile_id.clone(),
        merchant_connector_id: payment_attempt.merchant_connector_id.clone(),
        charges: None,
        split_refunds: req.split_refunds,
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
                creds_identifier,
                split_refunds,
                req.all_keys_required,
            ))
            .await?
        }
        Err(err) => {
            if err.current_context().is_db_unique_violation() {
                Err(errors::ApiErrorResponse::DuplicateRefundRequest)?
            } else {
                return Err(err)
                    .change_context(errors::ApiErrorResponse::RefundNotFound)
                    .attach_printable("Inserting Refund failed");
            }
        }
    };
    let unified_translated_message = if let (Some(unified_code), Some(unified_message)) =
        (refund.unified_code.clone(), refund.unified_message.clone())
    {
        helpers::get_unified_translation(
            state,
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

    Ok((refund, raw_connector_response).foreign_into())
}

// ********************************************** Refund list **********************************************

///   If payment-id is provided, lists all the refunds associated with that particular payment-id
///   If payment-id is not provided, lists the refunds associated with that particular merchant - to the limit specified,if no limits given, it is 10 by default
#[instrument(skip_all)]
#[cfg(feature = "olap")]
pub async fn refund_list(
    state: SessionState,
    platform: domain::Platform,
    profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
    req: api_models::refunds::RefundListRequest,
) -> RouterResponse<api_models::refunds::RefundListResponse> {
    let db = state.store;
    let limit = validator::validate_refund_list(req.limit)?;
    let offset = req.offset.unwrap_or_default();

    let refund_list = db
        .filter_refund_by_constraints(
            platform.get_processor().get_account().get_id(),
            &(req.clone(), profile_id_list.clone()).try_into()?,
            platform.get_processor().get_account().storage_scheme,
            limit,
            offset,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::RefundNotFound)?;

    let data: Vec<refunds::RefundResponse> = refund_list
        .into_iter()
        .map(ForeignInto::foreign_into)
        .collect();

    let total_count = db
        .get_total_count_of_refunds(
            platform.get_processor().get_account().get_id(),
            &(req, profile_id_list).try_into()?,
            platform.get_processor().get_account().storage_scheme,
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

#[instrument(skip_all)]
#[cfg(feature = "olap")]
pub async fn refund_filter_list(
    state: SessionState,
    platform: domain::Platform,
    req: common_utils::types::TimeRange,
) -> RouterResponse<api_models::refunds::RefundListMetaData> {
    let db = state.store;
    let filter_list = db
        .filter_refund_by_meta_constraints(
            platform.get_processor().get_account().get_id(),
            &req,
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::RefundNotFound)?;

    Ok(services::ApplicationResponse::Json(filter_list))
}

#[instrument(skip_all)]
pub async fn refund_retrieve_core_with_internal_reference_id(
    state: SessionState,
    platform: domain::Platform,
    profile_id: Option<common_utils::id_type::ProfileId>,
    refund_internal_request_id: String,
    force_sync: Option<bool>,
) -> RouterResult<diesel_refund::Refund> {
    let db = &*state.store;
    let merchant_id = platform.get_processor().get_account().get_id();

    let refund = db
        .find_refund_by_internal_reference_id_merchant_id(
            &refund_internal_request_id,
            merchant_id,
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::RefundNotFound)?;

    let request = refunds::RefundsRetrieveRequest {
        refund_id: refund.refund_id.clone(),
        force_sync,
        merchant_connector_details: None,
        all_keys_required: None,
    };

    Box::pin(refund_retrieve_core(
        state.clone(),
        platform,
        profile_id,
        request,
        refund,
    ))
    .await
    .map(|(refund, _)| refund)
}

#[instrument(skip_all)]
pub async fn refund_retrieve_core_with_refund_id(
    state: SessionState,
    platform: domain::Platform,
    profile_id: Option<common_utils::id_type::ProfileId>,
    request: refunds::RefundsRetrieveRequest,
) -> RouterResult<(diesel_refund::Refund, Option<masking::Secret<String>>)> {
    let refund_id = request.refund_id.clone();
    let db = &*state.store;
    let merchant_id = platform.get_processor().get_account().get_id();

    let refund = db
        .find_refund_by_merchant_id_refund_id(
            merchant_id,
            refund_id.as_str(),
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::RefundNotFound)?;

    Box::pin(refund_retrieve_core(
        state.clone(),
        platform,
        profile_id,
        request,
        refund,
    ))
    .await
}

#[instrument(skip_all)]
#[cfg(feature = "olap")]
pub async fn refund_manual_update(
    state: SessionState,
    req: api_models::refunds::RefundManualUpdateRequest,
) -> RouterResponse<serde_json::Value> {
    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            &req.merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        .attach_printable("Error while fetching the key store by merchant_id")?;
    let merchant_account = state
        .store
        .find_merchant_account_by_merchant_id(&req.merchant_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        .attach_printable("Error while fetching the merchant_account by merchant_id")?;
    let refund = state
        .store
        .find_refund_by_merchant_id_refund_id(
            merchant_account.get_id(),
            &req.refund_id,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::RefundNotFound)?;
    let refund_update = diesel_refund::RefundUpdate::ManualUpdate {
        refund_status: req.status.map(common_enums::RefundStatus::from),
        refund_error_message: req.error_message,
        refund_error_code: req.error_code,
        updated_by: merchant_account.storage_scheme.to_string(),
    };
    state
        .store
        .update_refund(
            refund.to_owned(),
            refund_update,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!(
                "Failed while updating refund: refund_id: {}",
                refund.refund_id
            )
        })?;
    Ok(services::ApplicationResponse::StatusOk)
}

#[instrument(skip_all)]
#[cfg(feature = "olap")]
pub async fn get_filters_for_refunds(
    state: SessionState,
    platform: domain::Platform,
    profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
) -> RouterResponse<api_models::refunds::RefundListFilters> {
    let merchant_connector_accounts = if let services::ApplicationResponse::Json(data) =
        super::admin::list_payment_connectors(
            state,
            platform.get_processor().get_account().get_id().to_owned(),
            profile_id_list,
        )
        .await?
    {
        data
    } else {
        return Err(errors::ApiErrorResponse::InternalServerError.into());
    };

    let connector_map = merchant_connector_accounts
        .into_iter()
        .filter_map(|merchant_connector_account| {
            merchant_connector_account
                .connector_label
                .clone()
                .map(|label| {
                    let info = merchant_connector_account.to_merchant_connector_info(&label);
                    (merchant_connector_account.connector_name, info)
                })
        })
        .fold(
            HashMap::new(),
            |mut map: HashMap<String, Vec<MerchantConnectorInfo>>, (connector_name, info)| {
                map.entry(connector_name).or_default().push(info);
                map
            },
        );

    Ok(services::ApplicationResponse::Json(
        api_models::refunds::RefundListFilters {
            connector: connector_map,
            currency: enums::Currency::iter().collect(),
            refund_status: enums::RefundStatus::iter().collect(),
        },
    ))
}

#[instrument(skip_all)]
#[cfg(feature = "olap")]
pub async fn get_aggregates_for_refunds(
    state: SessionState,
    platform: domain::Platform,
    profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
    time_range: common_utils::types::TimeRange,
) -> RouterResponse<api_models::refunds::RefundAggregateResponse> {
    let db = state.store.as_ref();
    let refund_status_with_count = db
        .get_refund_status_with_count(
            platform.get_processor().get_account().get_id(),
            profile_id_list,
            &time_range,
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to find status count")?;
    let mut status_map: HashMap<enums::RefundStatus, i64> =
        refund_status_with_count.into_iter().collect();
    for status in enums::RefundStatus::iter() {
        status_map.entry(status).or_default();
    }

    Ok(services::ApplicationResponse::Json(
        api_models::refunds::RefundAggregateResponse {
            status_with_count: status_map,
        },
    ))
}

impl ForeignFrom<diesel_refund::Refund> for api::RefundResponse {
    fn foreign_from(refund: diesel_refund::Refund) -> Self {
        let refund = refund;

        Self {
            payment_id: refund.payment_id,
            refund_id: refund.refund_id,
            amount: refund.refund_amount,
            currency: refund.currency.to_string(),
            reason: refund.refund_reason,
            status: refund.refund_status.foreign_into(),
            profile_id: refund.profile_id,
            metadata: refund.metadata,
            error_message: refund.refund_error_message,
            error_code: refund.refund_error_code,
            created_at: Some(refund.created_at),
            updated_at: Some(refund.modified_at),
            connector: refund.connector,
            merchant_connector_id: refund.merchant_connector_id,
            split_refunds: refund.split_refunds,
            unified_code: refund.unified_code,
            unified_message: refund.unified_message,
            issuer_error_code: refund.issuer_error_code,
            issuer_error_message: refund.issuer_error_message,
            raw_connector_response: None,
        }
    }
}

impl ForeignFrom<(diesel_refund::Refund, Option<masking::Secret<String>>)> for api::RefundResponse {
    fn foreign_from(item: (diesel_refund::Refund, Option<masking::Secret<String>>)) -> Self {
        let (refund, raw_connector_response) = item;

        Self {
            payment_id: refund.payment_id,
            refund_id: refund.refund_id,
            amount: refund.refund_amount,
            currency: refund.currency.to_string(),
            reason: refund.refund_reason,
            status: refund.refund_status.foreign_into(),
            profile_id: refund.profile_id,
            metadata: refund.metadata,
            error_message: refund.refund_error_message,
            error_code: refund.refund_error_code,
            created_at: Some(refund.created_at),
            updated_at: Some(refund.modified_at),
            connector: refund.connector,
            merchant_connector_id: refund.merchant_connector_id,
            split_refunds: refund.split_refunds,
            unified_code: refund.unified_code,
            unified_message: refund.unified_message,
            issuer_error_code: refund.issuer_error_code,
            issuer_error_message: refund.issuer_error_message,
            raw_connector_response,
        }
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
    creds_identifier: Option<String>,
    split_refunds: Option<SplitRefundsRequest>,
    all_keys_required: Option<bool>,
) -> RouterResult<(diesel_refund::Refund, Option<masking::Secret<String>>)> {
    // refunds::RefundResponse> {
    let db = &*state.store;
    let runner = storage::ProcessTrackerRunner::RefundWorkflowRouter;
    let task = "EXECUTE_REFUND";
    let task_id = format!("{runner}_{task}_{}", refund.internal_reference_id);

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
                                .attach_printable_lazy(|| format!("Failed while pushing refund execute task to scheduler, refund_id: {}", refund.refund_id))?;

                            Ok((refund, None))
                        }
                        api_models::refunds::RefundType::Instant => {
                            let update_refund = Box::pin(trigger_refund_to_gateway(
                                state,
                                &refund,
                                platform,
                                payment_attempt,
                                payment_intent,
                                creds_identifier,
                                split_refunds,
                                all_keys_required,
                            ))
                            .await;

                            match update_refund {
                                Ok((updated_refund_data, raw_connector_response)) => {
                                    add_refund_sync_task(db, &updated_refund_data, runner)
                                        .await
                                        .change_context(errors::ApiErrorResponse::InternalServerError)
                                        .attach_printable_lazy(|| format!(
                                            "Failed while pushing refund sync task in scheduler: refund_id: {}",
                                            refund.refund_id
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
                                .attach_printable_lazy(|| format!("Failed while pushing refund sync task in scheduler: refund_id: {}", refund.refund_id))?;
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

#[instrument(skip_all)]
pub async fn sync_refund_with_gateway_workflow(
    state: &SessionState,
    refund_tracker: &storage::ProcessTracker,
) -> Result<(), errors::ProcessTrackerError> {
    let refund_core = serde_json::from_value::<diesel_refund::RefundCoreWorkflow>(
        refund_tracker.tracking_data.clone(),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable_lazy(|| {
        format!(
            "unable to convert into refund_core {:?}",
            refund_tracker.tracking_data
        )
    })?;

    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            &refund_core.merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await?;

    let merchant_account = state
        .store
        .find_merchant_account_by_merchant_id(&refund_core.merchant_id, &key_store)
        .await?;
    let platform = domain::Platform::new(
        merchant_account.clone(),
        key_store.clone(),
        merchant_account.clone(),
        key_store.clone(),
    );
    let response = Box::pin(refund_retrieve_core_with_internal_reference_id(
        state.clone(),
        platform,
        None,
        refund_core.refund_internal_reference_id,
        Some(true),
    ))
    .await?;
    let terminal_status = [
        enums::RefundStatus::Success,
        enums::RefundStatus::Failure,
        enums::RefundStatus::TransactionFailure,
    ];
    match response.refund_status {
        status if terminal_status.contains(&status) => {
            state
                .store
                .as_scheduler()
                .finish_process_with_business_status(
                    refund_tracker.clone(),
                    business_status::COMPLETED_BY_PT,
                )
                .await?
        }
        _ => {
            _ = retry_refund_sync_task(
                &*state.store,
                response.connector,
                response.merchant_id,
                refund_tracker.to_owned(),
            )
            .await?;
        }
    }

    Ok(())
}

/// Schedule the task for refund retry
///
/// Returns bool which indicates whether this was the last refund retry or not
pub async fn retry_refund_sync_task(
    db: &dyn db::StorageInterface,
    connector: String,
    merchant_id: common_utils::id_type::MerchantId,
    pt: storage::ProcessTracker,
) -> Result<bool, sch_errors::ProcessTrackerError> {
    let schedule_time =
        get_refund_sync_process_schedule_time(db, &connector, &merchant_id, pt.retry_count + 1)
            .await?;

    match schedule_time {
        Some(s_time) => {
            db.as_scheduler().retry_process(pt, s_time).await?;
            Ok(false)
        }
        None => {
            db.as_scheduler()
                .finish_process_with_business_status(pt, business_status::RETRIES_EXCEEDED)
                .await?;
            Ok(true)
        }
    }
}

#[instrument(skip_all)]
pub async fn start_refund_workflow(
    state: &SessionState,
    refund_tracker: &storage::ProcessTracker,
) -> Result<(), errors::ProcessTrackerError> {
    match refund_tracker.name.as_deref() {
        Some("EXECUTE_REFUND") => {
            Box::pin(trigger_refund_execute_workflow(state, refund_tracker)).await
        }
        Some("SYNC_REFUND") => {
            Box::pin(sync_refund_with_gateway_workflow(state, refund_tracker)).await
        }
        _ => Err(errors::ProcessTrackerError::JobNotFound),
    }
}

#[instrument(skip_all)]
pub async fn trigger_refund_execute_workflow(
    state: &SessionState,
    refund_tracker: &storage::ProcessTracker,
) -> Result<(), errors::ProcessTrackerError> {
    let db = &*state.store;
    let refund_core = serde_json::from_value::<diesel_refund::RefundCoreWorkflow>(
        refund_tracker.tracking_data.clone(),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable_lazy(|| {
        format!(
            "unable to convert into refund_core {:?}",
            refund_tracker.tracking_data
        )
    })?;
    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            &refund_core.merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await?;

    let merchant_account = db
        .find_merchant_account_by_merchant_id(&refund_core.merchant_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let platform = domain::Platform::new(
        merchant_account.clone(),
        key_store.clone(),
        merchant_account.clone(),
        key_store.clone(),
    );
    let refund = db
        .find_refund_by_internal_reference_id_merchant_id(
            &refund_core.refund_internal_reference_id,
            &refund_core.merchant_id,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::RefundNotFound)?;
    match (&refund.sent_to_gateway, &refund.refund_status) {
        (false, enums::RefundStatus::Pending) => {
            let merchant_account = db
                .find_merchant_account_by_merchant_id(&refund.merchant_id, &key_store)
                .await
                .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

            let payment_attempt = db
                .find_payment_attempt_by_connector_transaction_id_payment_id_merchant_id(
                    &refund.connector_transaction_id,
                    &refund_core.payment_id,
                    &refund.merchant_id,
                    merchant_account.storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

            let payment_intent = db
                .find_payment_intent_by_payment_id_merchant_id(
                    &payment_attempt.payment_id,
                    &refund.merchant_id,
                    &key_store,
                    merchant_account.storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
            let split_refunds = core_utils::get_split_refunds(SplitRefundInput {
                split_payment_request: payment_intent.split_payments.clone(),
                payment_charges: payment_attempt.charges.clone(),
                charge_id: payment_attempt.charge_id.clone(),
                refund_request: refund.split_refunds.clone(),
            })?;

            //trigger refund request to gateway
            let (updated_refund, _) = Box::pin(trigger_refund_to_gateway(
                state,
                &refund,
                &platform,
                &payment_attempt,
                &payment_intent,
                None,
                split_refunds,
                None,
            ))
            .await?;
            add_refund_sync_task(
                db,
                &updated_refund,
                storage::ProcessTrackerRunner::RefundWorkflowRouter,
            )
            .await?;
        }
        (true, enums::RefundStatus::Pending) => {
            // create sync task
            add_refund_sync_task(
                db,
                &refund,
                storage::ProcessTrackerRunner::RefundWorkflowRouter,
            )
            .await?;
        }
        (_, _) => {
            //mark task as finished
            db.as_scheduler()
                .finish_process_with_business_status(
                    refund_tracker.clone(),
                    business_status::COMPLETED_BY_PT,
                )
                .await?;
        }
    };
    Ok(())
}

#[instrument]
pub fn refund_to_refund_core_workflow_model(
    refund: &diesel_refund::Refund,
) -> diesel_refund::RefundCoreWorkflow {
    diesel_refund::RefundCoreWorkflow {
        refund_internal_reference_id: refund.internal_reference_id.clone(),
        connector_transaction_id: refund.connector_transaction_id.clone(),
        merchant_id: refund.merchant_id.clone(),
        payment_id: refund.payment_id.clone(),
        processor_transaction_data: refund.processor_transaction_data.clone(),
    }
}

#[instrument(skip_all)]
pub async fn add_refund_sync_task(
    db: &dyn db::StorageInterface,
    refund: &diesel_refund::Refund,
    runner: storage::ProcessTrackerRunner,
) -> RouterResult<storage::ProcessTracker> {
    let task = "SYNC_REFUND";
    let process_tracker_id = format!("{runner}_{task}_{}", refund.internal_reference_id);
    let schedule_time =
        get_refund_sync_process_schedule_time(db, &refund.connector, &refund.merchant_id, 0)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to fetch schedule time for refund sync process")?
            .unwrap_or_else(common_utils::date_time::now);
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
                refund.refund_id
            )
        })?;
    metrics::TASKS_ADDED_COUNT.add(1, router_env::metric_attributes!(("flow", "Refund")));

    Ok(response)
}

#[instrument(skip_all)]
pub async fn add_refund_execute_task(
    db: &dyn db::StorageInterface,
    refund: &diesel_refund::Refund,
    runner: storage::ProcessTrackerRunner,
) -> RouterResult<storage::ProcessTracker> {
    let task = "EXECUTE_REFUND";
    let process_tracker_id = format!("{runner}_{task}_{}", refund.internal_reference_id);
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
                refund.refund_id
            )
        })?;
    Ok(response)
}

pub async fn get_refund_sync_process_schedule_time(
    db: &dyn db::StorageInterface,
    connector: &str,
    merchant_id: &common_utils::id_type::MerchantId,
    retry_count: i32,
) -> Result<Option<time::PrimitiveDateTime>, errors::ProcessTrackerError> {
    let mapping: common_utils::errors::CustomResult<
        process_data::ConnectorPTMapping,
        errors::StorageError,
    > = db
        .find_config_by_key(&format!("pt_mapping_refund_sync_{connector}"))
        .await
        .map(|value| value.config)
        .and_then(|config| {
            config
                .parse_struct("ConnectorPTMapping")
                .change_context(errors::StorageError::DeserializationFailed)
        });

    let mapping = match mapping {
        Ok(x) => x,
        Err(err) => {
            logger::error!("Error: while getting connector mapping: {err:?}");
            process_data::ConnectorPTMapping::default()
        }
    };

    let time_delta = process_tracker_utils::get_schedule_time(mapping, merchant_id, retry_count);

    Ok(process_tracker_utils::get_time_from_delta(time_delta))
}
