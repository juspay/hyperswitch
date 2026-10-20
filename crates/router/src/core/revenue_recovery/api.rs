use actix_web::{web, Responder};
use api_models::{payments as payments_api, payments as api_payments};
use common_utils::id_type;
use error_stack::{report, FutureExt, ResultExt};
use hyperswitch_domain_models::{payments as payments_domain, platform::Platform};

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

pub async fn call_proxy_api(
    state: &SessionState,
    payment_intent: &payments_domain::PaymentIntent,
    revenue_recovery_payment_data: &storage::revenue_recovery::RevenueRecoveryPaymentData,
    revenue_recovery: &payments_api::PaymentRevenueRecoveryMetadata,
    payment_processor_token: &str,
) -> RouterResult<payments_domain::PaymentConfirmData<api_types::Authorize>> {
    let operation = payments::operations::proxy_payments_intent::PaymentProxyIntent;
    let recurring_details = api_models::mandates::ProcessorPaymentToken {
        processor_payment_token: payment_processor_token.to_string(),
        merchant_connector_id: Some(revenue_recovery.get_merchant_connector_id_for_api_request()),
    };
    let req = payments_api::ProxyPaymentsRequest {
        return_url: None,
        amount: payments_api::AmountDetails::new(payment_intent.amount_details.clone().into()),
        recurring_details,
        shipping: None,
        browser_info: None,
        connector: revenue_recovery.connector.to_string(),
        merchant_connector_id: revenue_recovery.get_merchant_connector_id_for_api_request(),
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

    let request_payload = revenue_recovery_attempt_data
        .create_payment_record_request(
            state,
            &revenue_recovery_payment_data.billing_mca.id,
            Some(
                revenue_recovery_metadata
                    .active_attempt_payment_connector_id
                    .clone(),
            ),
            Some(revenue_recovery_metadata.connector),
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

/// Syncs the recorded attempt with the payment connector and returns the status it reports.
///
/// An attempt recorded for an external transaction is never the active attempt of the invoice,
/// and the invoice may already be in a final state, so this cannot go through the payment
/// retrieve API, which only syncs the active attempt of an invoice that is not final. The sync
/// is best effort: when the connector cannot be reached, or does not support syncing, `None` is
/// returned and the caller falls back to the reported status.
async fn sync_recorded_attempt_status(
    state: &SessionState,
    platform: &Platform,
    profile: &domain::Profile,
    payment_id: &id_type::GlobalPaymentId,
    attempt_id: &id_type::GlobalAttemptId,
) -> Option<common_enums::AttemptStatus> {
    let synced = call_psync_api_for_attempt(state, platform, profile, payment_id, attempt_id).await;
    match synced {
        Ok(payment_data) => Some(payment_data.payment_attempt.status),
        Err(error) => {
            logger::warn!(
                ?error,
                "Failed to sync the recorded attempt with the payment connector, using the reported transaction status"
            );
            None
        }
    }
}

/// Runs a payment connector sync for one particular attempt of an invoice, regardless of the
/// status of the invoice and of which attempt is active.
pub async fn call_psync_api_for_attempt(
    state: &SessionState,
    platform: &Platform,
    profile: &domain::Profile,
    payment_id: &id_type::GlobalPaymentId,
    attempt_id: &id_type::GlobalAttemptId,
) -> RouterResult<payments_domain::PaymentStatusData<api_types::PSync>> {
    let db = state.store.as_ref();
    let storage_scheme = platform.get_processor().get_account().storage_scheme;
    let key_store = platform.get_processor().get_key_store();

    let payment_intent = db
        .find_payment_intent_by_id(payment_id, key_store, storage_scheme)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
    let payment_attempt = db
        .find_payment_attempt_by_id(key_store, attempt_id, storage_scheme)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not find the recorded payment attempt")?;

    let payment_address = hyperswitch_domain_models::payment_address::PaymentAddress::new(
        payment_intent
            .shipping_address
            .clone()
            .map(|address| address.into_inner()),
        payment_intent
            .billing_address
            .clone()
            .map(|address| address.into_inner()),
        payment_attempt
            .payment_method_billing_address
            .clone()
            .map(|address| address.into_inner()),
        Some(true),
    );

    let get_tracker_response = payments::operations::GetTrackerResponse {
        payment_data: payments_domain::PaymentStatusData {
            flow: std::marker::PhantomData,
            payment_intent,
            payment_attempt,
            payment_address,
            attempts: None,
            should_sync_with_connector: true,
            merchant_connector_details: None,
        },
    };

    let req = payments_api::PaymentsRetrieveRequest {
        force_sync: true,
        param: None,
        expand_attempts: false,
        return_raw_connector_response: None,
        merchant_connector_details: None,
    };

    let (payment_data, _req, _, _, _, _) = Box::pin(payments::payments_operation_core::<
        api_types::PSync,
        _,
        _,
        _,
        payments_domain::PaymentStatusData<api_types::PSync>,
    >(
        state,
        state.get_req_state(),
        platform.clone(),
        profile,
        payments::operations::PaymentGet,
        req,
        get_tracker_response,
        payments::CallConnectorAction::Trigger,
        payments_domain::HeaderPayload::default(),
    ))
    .await?;
    Ok(payment_data)
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
    let store = state.store.as_ref();
    let payment_merchant_connector_account_id = request.payment_merchant_connector_id.to_owned();
    let profile_id = profile.get_id();
    // Find the payment & billing merchant connector id at the top level to avoid multiple DB calls.
    let payment_merchant_connector_account = store
        .find_merchant_connector_account_by_id(
            &payment_merchant_connector_account_id,
            platform.get_processor().get_key_store(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: payment_merchant_connector_account_id
                .clone()
                .get_string_repr()
                .to_string(),
        })?;

    utils::when(
        &payment_merchant_connector_account.profile_id != profile_id,
        || {
            Err(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                id: payment_merchant_connector_account_id
                    .clone()
                    .get_string_repr()
                    .to_string(),
            })
        },
    )?;
    let billing_connector_account = store
        .find_merchant_connector_account_by_id(
            &request.billing_merchant_connector_id.clone(),
            platform.get_processor().get_key_store(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: request
                .billing_merchant_connector_id
                .clone()
                .get_string_repr()
                .to_string(),
        })?;
    utils::when(&billing_connector_account.profile_id != profile_id, || {
        Err(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: request
                .billing_merchant_connector_id
                .clone()
                .get_string_repr()
                .to_string(),
        })
    })?;

    let recovery_intent =
        recovery_incoming::RevenueRecoveryInvoice::get_or_create_custom_recovery_intent(
            request.clone(),
            &state,
            &req_state,
            &platform,
            &profile,
        )
        .await
        .change_context(errors::ApiErrorResponse::GenericNotFoundError {
            message: format!(
                "Failed to load recovery intent for merchant reference id : {:?}",
                request.merchant_reference_id.to_owned()
            )
            .to_string(),
        })?;

    let (revenue_recovery_attempt_data, updated_recovery_intent) =
        recovery_incoming::RevenueRecoveryAttempt::load_recovery_attempt_from_api(
            request.clone(),
            &state,
            &req_state,
            &platform,
            &profile,
            recovery_intent.clone(),
            payment_merchant_connector_account,
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
        })?;

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

    // The reported `transaction_status` is only a hint: the status that the payment connector
    // returns for the transaction decides the action. The attempt was recorded with the reported
    // status, so it is what the decision falls back to when the connector cannot be synced.
    let mut revenue_recovery_attempt_data = revenue_recovery_attempt_data;
    let synced_attempt_status = sync_recorded_attempt_status(
        &state,
        &platform,
        &profile,
        &updated_recovery_intent.payment_id,
        &revenue_recovery_attempt_data.attempt_id,
    )
    .await;
    let attempt_status = match synced_attempt_status {
        Some(synced_status) => {
            if synced_status != revenue_recovery_attempt_data.attempt_status {
                router_env::logger::warn!(
                    reported_status = ?revenue_recovery_attempt_data.attempt_status,
                    ?synced_status,
                    "Status synced from the payment connector differs from the reported transaction status, using the synced status"
                );
            }
            revenue_recovery_attempt_data.attempt_status = synced_status;
            synced_status
        }
        None => revenue_recovery_attempt_data.attempt_status,
    };

    let recovery_action = recovery_incoming::RecoveryAction {
        action: recovery_incoming::RecoveryAction::get_action(
            recovery_incoming::RecoveryAction::get_event_for_attempt_status(attempt_status),
            Some(common_enums::TriggeredBy::External),
        ),
    };

    recovery_action
        .handle_action(
            &state,
            &profile,
            &platform,
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

    let response = api_models::payments::RecoveryPaymentsResponse {
        id: updated_recovery_intent.payment_id.to_owned(),
        intent_status: updated_recovery_intent.status.to_owned(),
        merchant_reference_id: updated_recovery_intent.merchant_reference_id.to_owned(),
    };

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        response,
    ))
}
