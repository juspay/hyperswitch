use actix_web::{web, Responder};
use api_models::{payments as payments_api, payments as api_payments};
use common_types::payments::{RecoveryConnectorAccounts, RecoveryConnectorDetails};
use common_utils::id_type;
use error_stack::{report, FutureExt, ResultExt};
use hyperswitch_domain_models::{payments as payments_domain, platform::Platform};

use super::{external_triggers, trigger_model::RecoveryTrigger};
use crate::{
    core::{
        errors::{self, RouterResult},
        payments::{self, operations::Operation},
        webhooks::recovery_incoming,
    },
    db::{
        errors::{RouterResponse, StorageErrorExt},
        storage::revenue_recovery_redis_operation::RedisTokenManager,
    },
    logger,
    routes::{app::ReqState, SessionState},
    services,
    types::{
        api::payments as api_types,
        domain,
        storage::{self, revenue_recovery as revenue_recovery_types},
        transformers::ForeignFrom,
    },
    utils,
};

pub async fn call_psync_api(
    state: &SessionState,
    global_payment_id: &id_type::GlobalPaymentId,
    revenue_recovery_data: &revenue_recovery_types::RevenueRecoveryPaymentData,
    force_sync_bool: bool,
    expand_attempts_bool: bool,
) -> RouterResult<payments_domain::PaymentStatusData<api_types::PSync>> {
    let operation = payments::operations::PaymentGet;
    let req = payments_api::PaymentsRetrieveRequest {
        force_sync: force_sync_bool,
        param: None,
        expand_attempts: expand_attempts_bool,
        return_raw_connector_response: None,
        merchant_connector_details: None,
    };
    let platform_from_revenue_recovery_data = Platform::new(
        revenue_recovery_data.merchant_account.clone(),
        revenue_recovery_data.key_store.clone(),
        revenue_recovery_data.merchant_account.clone(),
        revenue_recovery_data.key_store.clone(),
        None,
    );
    // TODO : Use api handler instead of calling get_tracker and payments_operation_core
    // Get the tracker related information. This includes payment intent and payment attempt
    let get_tracker_response = operation
        .to_get_tracker()?
        .get_trackers(
            state,
            global_payment_id,
            &req,
            &platform_from_revenue_recovery_data,
            &revenue_recovery_data.profile,
            &payments_domain::HeaderPayload::default(),
        )
        .await?;

    let (payment_data, _req, _, _, _, _) = Box::pin(payments::payments_operation_core::<
        api_types::PSync,
        _,
        _,
        _,
        payments_domain::PaymentStatusData<api_types::PSync>,
    >(
        state,
        state.get_req_state(),
        platform_from_revenue_recovery_data,
        &revenue_recovery_data.profile,
        operation,
        req,
        get_tracker_response,
        payments::CallConnectorAction::Trigger,
        payments_domain::HeaderPayload::default(),
    ))
    .await?;
    Ok(payment_data)
}

/// The connector accounts that the retries of an invoice run through. Retries run only under
/// internal triggers, where every transaction is recorded against them.
fn get_connector_accounts_for_retry(
    revenue_recovery_metadata: &payments_api::PaymentRevenueRecoveryMetadata,
) -> RouterResult<RecoveryConnectorAccounts> {
    revenue_recovery_metadata
        .get_connector_accounts()
        .ok_or_else(|| report!(errors::ApiErrorResponse::InternalServerError))
        .attach_printable("Connector accounts not found in revenue recovery metadata")
}

pub async fn call_proxy_api(
    state: &SessionState,
    payment_intent: &payments_domain::PaymentIntent,
    revenue_recovery_payment_data: &storage::revenue_recovery::RevenueRecoveryPaymentData,
    revenue_recovery: &payments_api::PaymentRevenueRecoveryMetadata,
    payment_processor_token: &str,
) -> RouterResult<payments_domain::PaymentConfirmData<api_types::Authorize>> {
    let operation = payments::operations::proxy_payments_intent::PaymentProxyIntent;
    let connector_accounts = get_connector_accounts_for_retry(revenue_recovery)?;
    let recurring_details = api_models::mandates::ProcessorPaymentToken {
        processor_payment_token: payment_processor_token.to_string(),
        merchant_connector_id: Some(connector_accounts.payment_connector_id.clone()),
    };
    let req = payments_api::ProxyPaymentsRequest {
        return_url: None,
        amount: payments_api::AmountDetails::new(payment_intent.amount_details.clone().into()),
        recurring_details,
        shipping: None,
        browser_info: None,
        connector: connector_accounts.connector.to_string(),
        merchant_connector_id: connector_accounts.payment_connector_id,
    };
    logger::info!(
        "Call made to payments proxy api , with the request body {:?}",
        req
    );
    let platform_from_revenue_recovery_payment_data = Platform::new(
        revenue_recovery_payment_data.merchant_account.clone(),
        revenue_recovery_payment_data.key_store.clone(),
        revenue_recovery_payment_data.merchant_account.clone(),
        revenue_recovery_payment_data.key_store.clone(),
        None,
    );

    // TODO : Use api handler instead of calling get_tracker and payments_operation_core
    // Get the tracker related information. This includes payment intent and payment attempt
    let get_tracker_response = operation
        .to_get_tracker()?
        .get_trackers(
            state,
            payment_intent.get_id(),
            &req,
            &platform_from_revenue_recovery_payment_data,
            &revenue_recovery_payment_data.profile,
            &payments_domain::HeaderPayload::default(),
        )
        .await?;

    let (payment_data, _req, _, _) = Box::pin(payments::proxy_for_payments_operation_core::<
        api_types::Authorize,
        _,
        _,
        _,
        payments_domain::PaymentConfirmData<api_types::Authorize>,
    >(
        state,
        state.get_req_state(),
        platform_from_revenue_recovery_payment_data,
        revenue_recovery_payment_data.profile.clone(),
        operation,
        req,
        get_tracker_response,
        payments::CallConnectorAction::Trigger,
        payments_domain::HeaderPayload::default(),
        None,
    ))
    .await?;
    Ok(payment_data)
}

pub async fn update_payment_intent_api(
    state: &SessionState,
    global_payment_id: id_type::GlobalPaymentId,
    revenue_recovery_payment_data: &storage::revenue_recovery::RevenueRecoveryPaymentData,
    update_req: payments_api::PaymentsUpdateIntentRequest,
) -> RouterResult<payments_domain::PaymentIntentData<api_types::PaymentUpdateIntent>> {
    // TODO : Use api handler instead of calling payments_intent_operation_core
    let operation = payments::operations::PaymentUpdateIntent;
    let platform_from_revenue_recovery_payment_data = Platform::new(
        revenue_recovery_payment_data.merchant_account.clone(),
        revenue_recovery_payment_data.key_store.clone(),
        revenue_recovery_payment_data.merchant_account.clone(),
        revenue_recovery_payment_data.key_store.clone(),
        None,
    );
    let (payment_data, _req, _) = Box::pin(payments::payments_intent_operation_core::<
        api_types::PaymentUpdateIntent,
        _,
        _,
        payments_domain::PaymentIntentData<api_types::PaymentUpdateIntent>,
    >(
        state,
        state.get_req_state(),
        platform_from_revenue_recovery_payment_data,
        revenue_recovery_payment_data.profile.clone(),
        operation,
        update_req,
        global_payment_id,
        payments_domain::HeaderPayload::default(),
    ))
    .await?;
    Ok(payment_data)
}

pub async fn record_internal_attempt_api(
    state: &SessionState,
    payment_intent: &payments_domain::PaymentIntent,
    revenue_recovery_payment_data: &storage::revenue_recovery::RevenueRecoveryPaymentData,
    revenue_recovery_metadata: &payments_api::PaymentRevenueRecoveryMetadata,
    card_info: payments_api::AdditionalCardInfo,
    payment_processor_token: &str,
) -> RouterResult<payments_api::PaymentAttemptRecordResponse> {
    let revenue_recovery_attempt_data =
        recovery_incoming::RevenueRecoveryAttempt::get_revenue_recovery_attempt(
            payment_intent,
            revenue_recovery_metadata,
            &revenue_recovery_payment_data.billing_mca,
            card_info,
            payment_processor_token,
        )
        .change_context(errors::ApiErrorResponse::GenericNotFoundError {
            message: "get_revenue_recovery_attempt was not constructed".to_string(),
        })?;

    let connector_accounts = get_connector_accounts_for_retry(revenue_recovery_metadata)?;

    let request_payload = revenue_recovery_attempt_data
        .create_payment_record_request(
            state,
            &RecoveryConnectorDetails::ConnectorAccounts(connector_accounts),
            common_enums::TriggeredBy::Internal,
        )
        .await
        .change_context(errors::ApiErrorResponse::GenericNotFoundError {
            message: "Cannot Create the payment record Request".to_string(),
        })?;

    let platform_from_revenue_recovery_payment_data = Platform::new(
        revenue_recovery_payment_data.merchant_account.clone(),
        revenue_recovery_payment_data.key_store.clone(),
        revenue_recovery_payment_data.merchant_account.clone(),
        revenue_recovery_payment_data.key_store.clone(),
        None,
    );

    let attempt_response = Box::pin(payments::record_attempt_core(
        state.clone(),
        state.get_req_state(),
        platform_from_revenue_recovery_payment_data,
        revenue_recovery_payment_data.profile.clone(),
        request_payload,
        payment_intent.id.clone(),
        hyperswitch_domain_models::payments::HeaderPayload::default(),
    ))
    .await;

    match attempt_response {
        Ok(services::ApplicationResponse::JsonWithHeaders((attempt_response, _))) => {
            Ok(attempt_response)
        }
        Ok(_) => Err(errors::ApiErrorResponse::PaymentNotFound)
            .attach_printable("Unexpected response from record attempt core"),
        error @ Err(_) => {
            router_env::logger::error!(?error);
            Err(errors::ApiErrorResponse::PaymentNotFound)
                .attach_printable("failed to record attempt for revenue recovery workflow")
        }
    }
}

/// Checks that `amount_captured` agrees with the reported `transaction_status`, so that the
/// attempt is never recorded with amounts that contradict its status.
fn validate_recovery_amounts(
    request: &api_models::payments::RecoveryPaymentsCreate,
) -> RouterResult<()> {
    use common_types::payments::RecoveryTransactionStatus;

    let order_amount = common_utils::types::MinorUnit::from(request.amount_details.order_amount());
    let invalid = |message: &str| {
        Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: message.to_owned(),
        }))
    };

    match (request.transaction_status, request.amount_captured) {
        (RecoveryTransactionStatus::PartialCharged, None) => {
            invalid("`amount_captured` is required when `transaction_status` is `partial_charged`")
        }
        (RecoveryTransactionStatus::PartialCharged, Some(captured))
            if captured <= common_utils::types::MinorUnit::zero() || captured >= order_amount =>
        {
            invalid(
                "`amount_captured` must be greater than zero and less than the order amount for a `partial_charged` transaction",
            )
        }
        (RecoveryTransactionStatus::Charged, Some(captured)) if captured != order_amount => {
            invalid("`amount_captured` must be equal to the order amount for a `charged` transaction")
        }
        (RecoveryTransactionStatus::Failure, Some(_)) => {
            invalid("`amount_captured` must not be sent for a `failure` transaction")
        }
        _ => Ok(()),
    }
}

pub async fn custom_revenue_recovery_core(
    state: SessionState,
    req_state: ReqState,
    platform: Platform,
    profile: domain::Profile,
    request: api_models::payments::RecoveryPaymentsCreate,
) -> RouterResponse<payments_api::RecoveryPaymentsResponse> {
    validate_recovery_amounts(&request)?;
    validate_customer_information(&request)?;

    let updated_recovery_intent = match RecoveryTrigger::from_profile_settings(
        profile.revenue_recovery_trigger_model,
        profile.revenue_recovery_config.as_ref(),
        &request,
    )? {
        RecoveryTrigger::Internal {
            billing_merchant_connector_id,
            payment_merchant_connector_id,
        } => {
            Box::pin(record_transaction_and_schedule_retries(
                &state,
                &req_state,
                &platform,
                &profile,
                &request,
                &billing_merchant_connector_id,
                &payment_merchant_connector_id,
            ))
            .await?
        }
        RecoveryTrigger::External {
            connector,
            retry_config,
        } => {
            Box::pin(external_triggers::record_reported_transaction(
                &state,
                &req_state,
                &platform,
                &profile,
                &request,
                connector,
                retry_config,
            ))
            .await?
        }
    };

    let response = api_models::payments::RecoveryPaymentsResponse {
        id: updated_recovery_intent.payment_id.to_owned(),
        intent_status: updated_recovery_intent.status.to_owned(),
        merchant_reference_id: updated_recovery_intent.merchant_reference_id.to_owned(),
    };

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        response,
    ))
}

/// Finds a connector account of the profile.
async fn find_connector_account_of_profile(
    state: &SessionState,
    platform: &Platform,
    profile: &domain::Profile,
    merchant_connector_id: &id_type::MerchantConnectorAccountId,
) -> RouterResult<domain::MerchantConnectorAccount> {
    let not_found = || errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
        id: merchant_connector_id.get_string_repr().to_string(),
    };

    let connector_account = state
        .store
        .find_merchant_connector_account_by_id(
            merchant_connector_id,
            platform.get_processor().get_key_store(),
        )
        .await
        .to_not_found_response(not_found())?;

    utils::when(&connector_account.profile_id != profile.get_id(), || {
        Err(not_found())
    })?;

    Ok(connector_account)
}

/// Records the transaction against the connector accounts and schedules its retries.
async fn record_transaction_and_schedule_retries(
    state: &SessionState,
    req_state: &ReqState,
    platform: &Platform,
    profile: &domain::Profile,
    request: &api_models::payments::RecoveryPaymentsCreate,
    billing_merchant_connector_id: &id_type::MerchantConnectorAccountId,
    payment_merchant_connector_id: &id_type::MerchantConnectorAccountId,
) -> RouterResult<hyperswitch_domain_models::revenue_recovery::RecoveryPaymentIntent> {
    let payment_connector_account =
        find_connector_account_of_profile(state, platform, profile, payment_merchant_connector_id)
            .await?;
    let billing_connector_account =
        find_connector_account_of_profile(state, platform, profile, billing_merchant_connector_id)
            .await?;

    let recovery_intent =
        get_or_create_invoice(state, req_state, platform, profile, request).await?;

    let (revenue_recovery_attempt_data, updated_recovery_intent) = Box::pin(record_transaction(
        state,
        req_state,
        platform,
        profile,
        request,
        &recovery_intent,
        &RecoveryConnectorDetails::ConnectorAccounts(RecoveryConnectorAccounts {
            billing_connector_id: billing_connector_account.get_id(),
            payment_connector_id: payment_connector_account.get_id(),
            connector: payment_connector_account.get_connector_name(),
        }),
    ))
    .await?;

    let intent_retry_count = updated_recovery_intent
        .feature_metadata
        .as_ref()
        .and_then(|metadata| metadata.get_retry_count())
        .ok_or(report!(errors::ApiErrorResponse::GenericNotFoundError {
            message: "Failed to fetch retry count from intent feature metadata".to_string(),
        }))?;

    router_env::logger::info!("Intent retry count: {:?}", intent_retry_count);
    let mca_retry_threshold = billing_connector_account
        .get_retry_threshold()
        .ok_or(report!(errors::ApiErrorResponse::GenericNotFoundError {
            message: "Failed to fetch retry threshold from billing merchant connector account"
                .to_string(),
        }))?;

    // Recording an external attempt already syncs it with the payment connector, and the status
    // of the recorded attempt is the synced one. It falls back to the reported
    // `transaction_status` when the connector cannot be synced.
    let attempt_status = revenue_recovery_attempt_data.attempt_status;

    let recovery_action = recovery_incoming::RecoveryAction {
        action: recovery_incoming::RecoveryAction::get_action(
            recovery_incoming::RecoveryAction::get_event_for_attempt_status(attempt_status),
            Some(common_enums::TriggeredBy::External),
        ),
    };

    recovery_action
        .handle_action(
            state,
            profile,
            platform,
            &billing_connector_account,
            mca_retry_threshold,
            intent_retry_count,
            &(
                Some(revenue_recovery_attempt_data),
                updated_recovery_intent.clone(),
            ),
        )
        .await
        .change_context(errors::ApiErrorResponse::GenericNotFoundError {
            message: "Unexpected response from recovery core".to_string(),
        })?;

    Ok(updated_recovery_intent)
}

/// Finds or creates the invoice of the transaction.
pub(super) async fn get_or_create_invoice(
    state: &SessionState,
    req_state: &ReqState,
    platform: &Platform,
    profile: &domain::Profile,
    request: &api_models::payments::RecoveryPaymentsCreate,
) -> RouterResult<hyperswitch_domain_models::revenue_recovery::RecoveryPaymentIntent> {
    recovery_incoming::RevenueRecoveryInvoice::get_or_create_custom_recovery_intent(
        request.clone(),
        state,
        req_state,
        platform,
        profile,
    )
    .await
    .change_context(errors::ApiErrorResponse::GenericNotFoundError {
        message: format!(
            "Failed to load recovery intent for merchant reference id : {:?}",
            request.merchant_reference_id.to_owned()
        )
        .to_string(),
    })
}

/// Records the transaction against the invoice.
pub(super) async fn record_transaction(
    state: &SessionState,
    req_state: &ReqState,
    platform: &Platform,
    profile: &domain::Profile,
    request: &api_models::payments::RecoveryPaymentsCreate,
    recovery_intent: &hyperswitch_domain_models::revenue_recovery::RecoveryPaymentIntent,
    connector_details: &RecoveryConnectorDetails,
) -> RouterResult<(
    hyperswitch_domain_models::revenue_recovery::RecoveryPaymentAttempt,
    hyperswitch_domain_models::revenue_recovery::RecoveryPaymentIntent,
)> {
    Box::pin(
        recovery_incoming::RevenueRecoveryAttempt::load_recovery_attempt_from_api(
            request,
            state,
            req_state,
            platform,
            profile,
            recovery_intent,
            connector_details,
        ),
    )
    .await
    .map_err(|error| {
        let api_error = match error.current_context() {
            errors::RevenueRecoveryError::PaymentAttemptAlreadyRecorded => {
                errors::ApiErrorResponse::GenericDuplicateError {
                    message: format!(
                        "The transaction {} is already recorded against the invoice {}",
                        request.connector_transaction_id,
                        request.merchant_reference_id.get_string_repr()
                    ),
                }
            }
            _ => errors::ApiErrorResponse::GenericNotFoundError {
                message: format!(
                    "Failed to load recovery attempt for merchant reference id : {:?}",
                    request.merchant_reference_id.to_owned()
                ),
            },
        };
        error.change_context(api_error)
    })
}

/// Cancel the revenue recovery workflow of the invoice identified by `merchant_reference_id`
/// within `profile`.
pub async fn cancel_revenue_recovery_core(
    state: SessionState,
    platform: Platform,
    profile: domain::Profile,
    merchant_reference_id: id_type::PaymentReferenceId,
) -> RouterResponse<api_payments::RecoveryPaymentsResponse> {
    let db = &*state.store;
    let storage_scheme = platform.get_processor().get_storage_scheme();

    let payment_intent = db
        .find_payment_intent_by_merchant_reference_id_profile_id(
            &merchant_reference_id,
            profile.get_id(),
            platform.get_processor().get_key_store(),
            &storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    let cancelled_intent = super::cancel_invoice_workflows(
        &state,
        platform.get_processor().get_key_store(),
        storage_scheme,
        payment_intent,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("failed to cancel the revenue recovery workflow")?
    .map_err(|refusal| {
        report!(errors::ApiErrorResponse::PreconditionFailed {
            message: refusal.to_string(),
        })
    })?;

    let response = api_payments::RecoveryPaymentsResponse {
        id: cancelled_intent.id,
        intent_status: cancelled_intent.status,
        merchant_reference_id: cancelled_intent.merchant_reference_id,
    };

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        response,
    ))
}

fn validate_customer_information(
    request: &api_models::payments::RecoveryPaymentsCreate,
) -> RouterResult<()> {
    if let Some(document) = request
        .customer_details
        .as_ref()
        .and_then(|customer| customer.document_details.as_ref())
    {
        document
            .validate()
            .map_err(|err| errors::ApiErrorResponse::PreconditionFailed {
                message: err.to_string(),
            })?;
    }

    Ok(())
}
