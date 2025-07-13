use api_models::payments as payments_api;
use common_utils::id_type;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    merchant_context::{Context, MerchantContext},
    payments as payments_domain,
};

use crate::{
    core::{
        errors::{self, RouterResult},
        payments::{self, operations::Operation},
        webhooks::recovery_incoming,
    },
    logger,
    routes::SessionState,
    services,
    types::{
        api::payments as api_types,
        storage::{self, revenue_recovery as revenue_recovery_types},
    },
};

pub async fn call_psync_api(
    state: &SessionState,
    global_payment_id: &id_type::GlobalPaymentId,
    revenue_recovery_data: &revenue_recovery_types::RevenueRecoveryPaymentData,
) -> RouterResult<payments_domain::PaymentStatusData<api_types::PSync>> {
    let operation = payments::operations::PaymentGet;
    let req = payments_api::PaymentsRetrieveRequest {
        force_sync: false,
        param: None,
        expand_attempts: true,
        return_raw_connector_response: None,
        merchant_connector_details: None,
    };
    let merchant_context_from_revenue_recovery_data =
        MerchantContext::NormalMerchant(Box::new(Context(
            revenue_recovery_data.merchant_account.clone(),
            revenue_recovery_data.key_store.clone(),
        )));
    // TODO : Use api handler instead of calling get_tracker and payments_operation_core
    // Get the tracker related information. This includes payment intent and payment attempt
    let get_tracker_response = operation
        .to_get_tracker()?
        .get_trackers(
            state,
            global_payment_id,
            &req,
            &merchant_context_from_revenue_recovery_data,
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
        merchant_context_from_revenue_recovery_data,
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
) -> RouterResult<payments_domain::PaymentConfirmData<api_types::Authorize>> {
    let operation = payments::operations::proxy_payments_intent::PaymentProxyIntent;
    let req = payments_api::ProxyPaymentsRequest {
        return_url: None,
        amount: payments_api::AmountDetails::new(payment_intent.amount_details.clone().into()),
        recurring_details: revenue_recovery.get_payment_token_for_api_request(),
        shipping: None,
        browser_info: None,
        connector: revenue_recovery.connector.to_string(),
        merchant_connector_id: revenue_recovery.get_merchant_connector_id_for_api_request(),
    };
    logger::info!(
        "Call made to payments proxy api , with the request body {:?}",
        req
    );
    let merchant_context_from_revenue_recovery_payment_data =
        MerchantContext::NormalMerchant(Box::new(Context(
            revenue_recovery_payment_data.merchant_account.clone(),
            revenue_recovery_payment_data.key_store.clone(),
        )));

    // TODO : Use api handler instead of calling get_tracker and payments_operation_core
    // Get the tracker related information. This includes payment intent and payment attempt
    let get_tracker_response = operation
        .to_get_tracker()?
        .get_trackers(
            state,
            payment_intent.get_id(),
            &req,
            &merchant_context_from_revenue_recovery_payment_data,
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
        merchant_context_from_revenue_recovery_payment_data,
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
    let merchant_context_from_revenue_recovery_payment_data =
        MerchantContext::NormalMerchant(Box::new(Context(
            revenue_recovery_payment_data.merchant_account.clone(),
            revenue_recovery_payment_data.key_store.clone(),
        )));
    let (payment_data, _req, _) = payments::payments_intent_operation_core::<
        api_types::PaymentUpdateIntent,
        _,
        _,
        payments_domain::PaymentIntentData<api_types::PaymentUpdateIntent>,
    >(
        state,
        state.get_req_state(),
        merchant_context_from_revenue_recovery_payment_data,
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
) -> RouterResult<payments_api::PaymentAttemptRecordResponse> {
    let revenue_recovery_attempt_data =
        recovery_incoming::RevenueRecoveryAttempt::get_revenue_recovery_attempt(
            payment_intent,
            revenue_recovery_metadata,
            &revenue_recovery_payment_data.billing_mca,
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

    let merchant_context_from_revenue_recovery_payment_data =
        MerchantContext::NormalMerchant(Box::new(Context(
            revenue_recovery_payment_data.merchant_account.clone(),
            revenue_recovery_payment_data.key_store.clone(),
        )));

    let attempt_response = Box::pin(payments::record_attempt_core(
        state.clone(),
        state.get_req_state(),
        merchant_context_from_revenue_recovery_payment_data,
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
