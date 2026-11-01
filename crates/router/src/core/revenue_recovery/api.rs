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

/// The payment connector account and connector that the retries of an invoice run through. Only
/// invoices whose transactions are recorded with connector accounts are retried, so these are
/// always present here.
fn get_payment_connector_details(
    revenue_recovery_metadata: &payments_api::PaymentRevenueRecoveryMetadata,
) -> RouterResult<(
    id_type::MerchantConnectorAccountId,
    common_enums::connector_enums::Connector,
)> {
    revenue_recovery_metadata
        .active_attempt_payment_connector_id
        .clone()
        .zip(revenue_recovery_metadata.connector)
        .ok_or_else(|| report!(errors::ApiErrorResponse::InternalServerError))
        .attach_printable("Payment connector details not found in revenue recovery metadata")
}

pub async fn call_proxy_api(
    state: &SessionState,
    payment_intent: &payments_domain::PaymentIntent,
    revenue_recovery_payment_data: &storage::revenue_recovery::RevenueRecoveryPaymentData,
    revenue_recovery: &payments_api::PaymentRevenueRecoveryMetadata,
    payment_processor_token: &str,
) -> RouterResult<payments_domain::PaymentConfirmData<api_types::Authorize>> {
    let operation = payments::operations::proxy_payments_intent::PaymentProxyIntent;
    let (merchant_connector_id, connector) = get_payment_connector_details(revenue_recovery)?;
    let recurring_details = api_models::mandates::ProcessorPaymentToken {
        processor_payment_token: payment_processor_token.to_string(),
        merchant_connector_id: Some(merchant_connector_id.clone()),
    };
    let req = payments_api::ProxyPaymentsRequest {
        return_url: None,
        amount: payments_api::AmountDetails::new(payment_intent.amount_details.clone().into()),
        recurring_details,
        shipping: None,
        browser_info: None,
        connector: connector.to_string(),
        merchant_connector_id,
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

    let (payment_merchant_connector_id, connector) =
        get_payment_connector_details(revenue_recovery_metadata)?;

    let request_payload = revenue_recovery_attempt_data
        .create_payment_record_request(
            state,
            Some(&revenue_recovery_payment_data.billing_mca.id),
            Some(payment_merchant_connector_id),
            Some(connector),
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

    match profile.revenue_recovery_trigger_model {
        common_enums::RevenueRecoveryTriggerModel::InternalTriggers => {
            let (billing_merchant_connector_id, payment_merchant_connector_account_id) =
                get_connector_accounts_for_internal_triggers(&request)?;
            Box::pin(record_transaction_for_internal_triggers(
                state,
                req_state,
                platform,
                profile,
                request,
                billing_merchant_connector_id,
                payment_merchant_connector_account_id,
            ))
            .await
        }
        common_enums::RevenueRecoveryTriggerModel::ExternalTriggers => {
            validate_connector_fields_for_external_triggers(&request)?;
            Box::pin(record_transaction_for_external_triggers(
                state, req_state, platform, profile, request,
            ))
            .await
        }
    }
}

/// With internal triggers, the retries run through both connector accounts and the processor comes
/// from the payment account.
fn get_connector_accounts_for_internal_triggers(
    request: &api_models::payments::RecoveryPaymentsCreate,
) -> RouterResult<(
    id_type::MerchantConnectorAccountId,
    id_type::MerchantConnectorAccountId,
)> {
    utils::when(request.connector.is_some(), || {
        Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "`connector` must not be sent when the `revenue_recovery_trigger_model` of the profile is `internal_triggers`".to_owned(),
        })
    })?;

    let billing_merchant_connector_id =
        request
            .billing_merchant_connector_id
            .clone()
            .ok_or_else(|| {
                report!(errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "billing_merchant_connector_id".into(),
                })
            })?;
    let payment_merchant_connector_id =
        request
            .payment_merchant_connector_id
            .clone()
            .ok_or_else(|| {
                report!(errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "payment_merchant_connector_id".into(),
                })
            })?;

    Ok((billing_merchant_connector_id, payment_merchant_connector_id))
}

/// With external triggers there are no connector accounts, so the reported processor is all that
/// is known about the connector.
fn validate_connector_fields_for_external_triggers(
    request: &api_models::payments::RecoveryPaymentsCreate,
) -> RouterResult<()> {
    utils::when(
        request.billing_merchant_connector_id.is_some()
            || request.payment_merchant_connector_id.is_some(),
        || {
            Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "`billing_merchant_connector_id` and `payment_merchant_connector_id` must not be sent when the `revenue_recovery_trigger_model` of the profile is `external_triggers`".to_owned(),
            })
        },
    )?;

    utils::when(request.connector.is_none(), || {
        Err(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "connector".into(),
        })
    })?;

    Ok(())
}

/// Records the transaction and leaves everything else to the merchant, who triggers the retries.
async fn record_transaction_for_external_triggers(
    state: SessionState,
    req_state: ReqState,
    platform: Platform,
    profile: domain::Profile,
    request: api_models::payments::RecoveryPaymentsCreate,
) -> RouterResponse<payments_api::RecoveryPaymentsResponse> {
    let (_, updated_recovery_intent) = Box::pin(record_transaction(
        &state, &req_state, &platform, &profile, &request, None,
    ))
    .await?;

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        get_recovery_payments_response(&updated_recovery_intent),
    ))
}

/// Records the transaction against the connector accounts and schedules its retries.
async fn record_transaction_for_internal_triggers(
    state: SessionState,
    req_state: ReqState,
    platform: Platform,
    profile: domain::Profile,
    request: api_models::payments::RecoveryPaymentsCreate,
    billing_merchant_connector_id: id_type::MerchantConnectorAccountId,
    payment_merchant_connector_account_id: id_type::MerchantConnectorAccountId,
) -> RouterResponse<payments_api::RecoveryPaymentsResponse> {
    let store = state.store.as_ref();
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
            &billing_merchant_connector_id,
            platform.get_processor().get_key_store(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: billing_merchant_connector_id
                .clone()
                .get_string_repr()
                .to_string(),
        })?;
    utils::when(&billing_connector_account.profile_id != profile_id, || {
        Err(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: billing_merchant_connector_id
                .clone()
                .get_string_repr()
                .to_string(),
        })
    })?;

    let (revenue_recovery_attempt_data, updated_recovery_intent) = Box::pin(record_transaction(
        &state,
        &req_state,
        &platform,
        &profile,
        &request,
        Some(payment_merchant_connector_account),
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

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        get_recovery_payments_response(&updated_recovery_intent),
    ))
}

fn get_recovery_payments_response(
    recovery_intent: &hyperswitch_domain_models::revenue_recovery::RecoveryPaymentIntent,
) -> payments_api::RecoveryPaymentsResponse {
    payments_api::RecoveryPaymentsResponse {
        id: recovery_intent.payment_id.to_owned(),
        intent_status: recovery_intent.status.to_owned(),
        merchant_reference_id: recovery_intent.merchant_reference_id.to_owned(),
    }
}

/// Finds or creates the invoice of the transaction, and records the transaction against it.
async fn record_transaction(
    state: &SessionState,
    req_state: &ReqState,
    platform: &Platform,
    profile: &domain::Profile,
    request: &api_models::payments::RecoveryPaymentsCreate,
    payment_merchant_connector_account: Option<domain::MerchantConnectorAccount>,
) -> RouterResult<(
    hyperswitch_domain_models::revenue_recovery::RecoveryPaymentAttempt,
    hyperswitch_domain_models::revenue_recovery::RecoveryPaymentIntent,
)> {
    validate_customer_information(request)?;

    let recovery_intent =
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
        })?;

    Box::pin(
        recovery_incoming::RevenueRecoveryAttempt::load_recovery_attempt_from_api(
            request.clone(),
            state,
            req_state,
            platform,
            profile,
            recovery_intent.clone(),
            payment_merchant_connector_account,
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

#[cfg(test)]
mod connector_field_tests {
    use super::{
        errors::ApiErrorResponse, get_connector_accounts_for_internal_triggers,
        validate_connector_fields_for_external_triggers,
    };

    fn recovery_request(
        connector_fields: serde_json::Value,
    ) -> api_models::payments::RecoveryPaymentsCreate {
        let mut request = serde_json::json!({
            "amount_details": { "order_amount": 6540, "currency": "USD" },
            "merchant_reference_id": "invoice_mbabizu24mvu3mela5njyh",
            "transaction_status": "failure",
            "payment_method_type": "card",
            "payment_method_sub_type": "credit",
            "connector_customer_id": "cust_12345",
            "connector_transaction_id": "993672945374576J",
            "transaction_created_at": "2022-09-10T10:11:12Z",
            "billing_started_at": "2022-09-10T10:11:12Z",
            "payment_method_data": {
                "primary_processor_payment_method_token": "token_1234",
                "payment_method_metadata": {
                    "card_network": "Visa",
                    "card_type": "credit",
                    "last4": "4242",
                    "card_isin": "424242",
                    "card_extended_bin": "42424242",
                    "card_exp_month": "01",
                    "card_exp_year": "2026"
                }
            }
        });
        if let (Some(request), Some(connector_fields)) =
            (request.as_object_mut(), connector_fields.as_object())
        {
            request.extend(connector_fields.clone());
        }
        serde_json::from_value(request).expect("valid recovery request")
    }

    fn is_missing_field(error: &ApiErrorResponse, field: &str) -> bool {
        matches!(
            error,
            ApiErrorResponse::MissingRequiredField { field_name } if *field_name == field
        )
    }

    #[test]
    fn internal_triggers_need_the_billing_account() {
        let request = recovery_request(serde_json::json!({
            "payment_merchant_connector_id": "mca_payment",
        }));

        let error = get_connector_accounts_for_internal_triggers(&request)
            .expect_err("billing account is required");

        assert!(is_missing_field(
            error.current_context(),
            "billing_merchant_connector_id"
        ));
    }

    #[test]
    fn internal_triggers_need_the_payment_account() {
        let request = recovery_request(serde_json::json!({
            "billing_merchant_connector_id": "mca_billing",
        }));

        let error = get_connector_accounts_for_internal_triggers(&request)
            .expect_err("payment account is required");

        assert!(is_missing_field(
            error.current_context(),
            "payment_merchant_connector_id"
        ));
    }

    #[test]
    fn internal_triggers_do_not_accept_a_reported_connector() {
        let request = recovery_request(serde_json::json!({
            "billing_merchant_connector_id": "mca_billing",
            "payment_merchant_connector_id": "mca_payment",
            "connector": "stripe",
        }));

        let error = get_connector_accounts_for_internal_triggers(&request)
            .expect_err("the processor comes from the payment account");

        assert!(matches!(
            error.current_context(),
            ApiErrorResponse::InvalidRequestData { .. }
        ));
    }

    #[test]
    fn internal_triggers_use_the_accounts_of_the_request() {
        let request = recovery_request(serde_json::json!({
            "billing_merchant_connector_id": "mca_billing",
            "payment_merchant_connector_id": "mca_payment",
        }));

        let (billing_account, payment_account) =
            get_connector_accounts_for_internal_triggers(&request).expect("both accounts are sent");

        assert_eq!(billing_account.get_string_repr(), "mca_billing");
        assert_eq!(payment_account.get_string_repr(), "mca_payment");
    }

    #[test]
    fn external_triggers_need_the_connector() {
        let request = recovery_request(serde_json::json!({}));

        let error = validate_connector_fields_for_external_triggers(&request)
            .expect_err("the connector is required");

        assert!(is_missing_field(error.current_context(), "connector"));
    }

    #[test]
    fn external_triggers_do_not_accept_connector_accounts() {
        for account_field in [
            "billing_merchant_connector_id",
            "payment_merchant_connector_id",
        ] {
            let request = recovery_request(serde_json::json!({
                "connector": "stripe",
                account_field: "mca_1234567890",
            }));

            let error = validate_connector_fields_for_external_triggers(&request)
                .expect_err("no account is used with external triggers");

            assert!(matches!(
                error.current_context(),
                ApiErrorResponse::InvalidRequestData { .. }
            ));
        }
    }

    #[test]
    fn external_triggers_accept_every_listed_connector() {
        for connector in ["adyen", "stripe", "paypal", "other"] {
            let request = recovery_request(serde_json::json!({ "connector": connector }));

            assert!(validate_connector_fields_for_external_triggers(&request).is_ok());
        }
    }
}
