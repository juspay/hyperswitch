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
    );
    let (payment_data, _req, _) = payments::payments_intent_operation_core::<
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
    )
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

pub async fn custom_revenue_recovery_core(
    state: SessionState,
    req_state: ReqState,
    platform: Platform,
    profile: domain::Profile,
    request: api_models::payments::RecoveryPaymentsCreate,
) -> RouterResponse<payments_api::RecoveryPaymentsResponse> {
    let store = state.store.as_ref();
    let payment_merchant_connector_account_id = request.payment_merchant_connector_id.to_owned();
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
        .change_context(errors::ApiErrorResponse::GenericNotFoundError {
            message: format!(
                "Failed to load recovery attempt for merchant reference id : {:?}",
                request.merchant_reference_id.to_owned()
            )
            .to_string(),
        })?;

    let intent_retry_count = updated_recovery_intent
        .feature_metadata
        .as_ref()
        .and_then(|metadata| metadata.get_retry_count())
        .ok_or(report!(errors::ApiErrorResponse::GenericNotFoundError {
            message: "Failed to fetch retry count from intent feature metadata".to_string(),
        }))?;

    router_env::logger::info!("Intent retry count: {:?}", intent_retry_count);
    let recovery_action = recovery_incoming::RecoveryAction {
        action: request.action.to_owned(),
    };
    let mca_retry_threshold = billing_connector_account
        .get_retry_threshold()
        .ok_or(report!(errors::ApiErrorResponse::GenericNotFoundError {
            message: "Failed to fetch retry threshold from billing merchant connector account"
                .to_string(),
        }))?;

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
