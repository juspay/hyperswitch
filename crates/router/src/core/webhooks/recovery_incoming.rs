use api_models::webhooks::{self, WebhookResponseTracker};
use common_utils::{transformers::ForeignFrom, types::MinorUnit};
use error_stack::{report, ResultExt};
use hyperswitch_interfaces::{
    recovery::{
        RecoveryAction, RevenueRecoveryAction, RevenueRecoveryInvoiceData,
        RevenueRecoveryTransactionData,
    },
    webhooks::IncomingWebhookRequestDetails,
};
use router_env::{instrument, tracing};

use crate::{
    core::{
        api_locking,
        errors::{self, CustomResult},
        payments::{self, operations},
    },
    routes::{app::ReqState, SessionState},
    services::{self, connector_integration_interface},
    types::{
        api::{self, IncomingWebhook},
        domain,
    },
};

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
#[cfg(feature = "revenue_recovery")]
pub async fn recovery_incoming_webhook_flow(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    business_profile: domain::Profile,
    key_store: domain::MerchantKeyStore,
    _webhook_details: api::IncomingWebhookDetails,
    source_verified: bool,
    connector: &connector_integration_interface::ConnectorEnum,
    request_details: &IncomingWebhookRequestDetails<'_>,
    event_type: webhooks::IncomingWebhookEvent,
    req_state: ReqState,
) -> CustomResult<WebhookResponseTracker, errors::RevenueRecoveryError> {
    common_utils::fp_utils::when(source_verified, || {
        Err(report!(
            errors::RevenueRecoveryError::WebhookAuthenticationFailed
        ))
    })?;

    let invoice_details = connector
        .get_revenue_recovery_invoice_details(request_details)
        .change_context(errors::RevenueRecoveryError::InvoiceWebhookProcessingFailed)
        .attach_printable("Failed while getting revenue recovery invoice details")?;
    // Fetch the intent using merchant reference id, if not found create new intent.
    let payment_intent = invoice_details
        .get_payment_intent(
            &state,
            &req_state,
            &merchant_account,
            &business_profile,
            &key_store,
        )
        .await?
        .unwrap_or(
            invoice_details
                .create_payment_intent(
                    &state,
                    &req_state,
                    &merchant_account,
                    &business_profile,
                    &key_store,
                )
                .await?,
        );

    let payment_attempt = if event_type.is_recovery_transaction_event() {
        let invoice_transaction_details = connector
            .get_revenue_recovery_transaction_details(request_details)
            .change_context(errors::RevenueRecoveryError::TransactionWebhookProcessingFailed)?;
        // Record attempt logic needs to be added when attempt is not found.
        invoice_transaction_details
            .get_payment_attempt(
                &state,
                &req_state,
                &merchant_account,
                &business_profile,
                &key_store,
                payment_intent.payment_id.clone(),
            )
            .await?
    } else {
        None
    };
    let attempt_triggered_by = payment_attempt.and_then(|attempt| {
        attempt.feature_metadata.and_then(|metadata| {
            metadata
                .revenue_recovery
                .map(|recovery| recovery.attempt_triggered_by)
        })
    });

    let action = RecoveryAction::find_action(event_type, attempt_triggered_by);

    match action {
        RecoveryAction::CancelInvoice => todo!(),
        RecoveryAction::ScheduleFailedPayment => todo!(),
        RecoveryAction::SuccessPaymentExternal => todo!(),
        RecoveryAction::PendingPayment => {
            router_env::logger::info!(
                "Pending transactions are not consumed by the revenue recovery webhooks"
            );
            Ok(WebhookResponseTracker::NoEffect)
        }
        RecoveryAction::NoAction => {
            router_env::logger::info!(
                "No Recovery action is taken place for recovery event : {:?} and attempt triggered_by : {:?} ", event_type.clone(), attempt_triggered_by
            );
            Ok(WebhookResponseTracker::NoEffect)
        }
        RecoveryAction::InvalidAction => {
            router_env::logger::error!(
                "Invalid Revenue recovery action state has been received, event : {:?}, triggered_by : {:?}", event_type, attempt_triggered_by
            );
            Ok(WebhookResponseTracker::NoEffect)
        }
    }
}

// Intent related functions for the invoice are implemented in this trait
pub trait RevenueRecoveryInvoice {
    /// get the payment intent using merchant reference id.
    async fn get_payment_intent(
        &self,
        state: &SessionState,
        req_state: &ReqState,
        merchant_account: &domain::MerchantAccount,
        profile: &domain::Profile,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Option<RecoveryPaymentIntent>, errors::RevenueRecoveryError>;
    /// create payment intent if intent was not found for merchant reference id.
    async fn create_payment_intent(
        &self,
        state: &SessionState,
        req_state: &ReqState,
        merchant_account: &domain::MerchantAccount,
        profile: &domain::Profile,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<RecoveryPaymentIntent, errors::RevenueRecoveryError>;
}

/// Attempt related functions for the invoice transactions are implemented in this trait
pub trait RevenueRecoveryTransaction {
    /// Get the payment attempt using connector transaction id.
    async fn get_payment_attempt(
        &self,
        state: &SessionState,
        req_state: &ReqState,
        merchant_account: &domain::MerchantAccount,
        profile: &domain::Profile,
        key_store: &domain::MerchantKeyStore,
        payment_id: common_utils::id_type::GlobalPaymentId,
    ) -> CustomResult<Option<RecoveryPaymentAttempt>, errors::RevenueRecoveryError>;
    /// record payment attempt against given intent.
    async fn record_payment_attempt(
        &self,
        state: &SessionState,
        req_state: &ReqState,
        merchant_account: &domain::MerchantAccount,
        profile: &domain::Profile,
        key_store: &domain::MerchantKeyStore,
        payment_id: common_utils::id_type::GlobalPaymentId,
    ) -> CustomResult<RecoveryPaymentAttempt, errors::RevenueRecoveryError>;
}

pub struct RecoveryPaymentIntent {
    pub payment_id: common_utils::id_type::GlobalPaymentId,
    pub status: common_enums::enums::IntentStatus,
    pub feature_metadata: Option<api_models::payments::FeatureMetadata>,
}

pub struct RecoveryPaymentAttempt {
    pub attempt_id: common_utils::id_type::GlobalAttemptId,
    pub attempt_status: common_enums::AttemptStatus,
    pub feature_metadata: Option<api_models::payments::PaymentAttemptFeatureMetadata>,
}

/// Implement the trait for RevenueRecoveryTransactionData
impl RevenueRecoveryInvoice for RevenueRecoveryInvoiceData {
    async fn get_payment_intent(
        &self,
        state: &SessionState,
        req_state: &ReqState,
        merchant_account: &domain::MerchantAccount,
        profile: &domain::Profile,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Option<RecoveryPaymentIntent>, errors::RevenueRecoveryError> {
        let payment_response = Box::pin(payments::payments_get_intent_using_merchant_reference(
            state.clone(),
            merchant_account.clone(),
            profile.clone(),
            key_store.clone(),
            req_state.clone(),
            &self.merchant_reference_id,
            hyperswitch_domain_models::payments::HeaderPayload::default(),
            None,
        ))
        .await;
        router_env::logger::info!(?payment_response);
        let response = match payment_response {
            Ok(services::ApplicationResponse::JsonWithHeaders((payments_response, _))) => {
                let payment_id = payments_response.id.clone();
                let status = payments_response.status;
                let feature_metadata = payments_response.feature_metadata;
                Ok(Some(RecoveryPaymentIntent {
                    payment_id,
                    status,
                    feature_metadata,
                }))
            }
            Err(err)
                if matches!(
                    err.current_context(),
                    &errors::ApiErrorResponse::PaymentNotFound
                ) =>
            {
                Ok(None)
            }
            Ok(_) | Err(_) => Err(errors::RevenueRecoveryError::PaymentIntentFetchFailed)
                .attach_printable("failed to fetch payment intent recovery webhook flow"),
        }?;
        Ok(response)
    }
    async fn create_payment_intent(
        &self,
        state: &SessionState,
        req_state: &ReqState,
        merchant_account: &domain::MerchantAccount,
        profile: &domain::Profile,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<RecoveryPaymentIntent, errors::RevenueRecoveryError> {
        let payload = api_models::payments::PaymentsCreateIntentRequest::from(self);
        let global_payment_id =
            common_utils::id_type::GlobalPaymentId::generate(&state.conf.cell_information.id);

        let create_intent_response = Box::pin(payments::payments_intent_core::<
            hyperswitch_domain_models::router_flow_types::payments::PaymentCreateIntent,
            api_models::payments::PaymentsIntentResponse,
            _,
            _,
            hyperswitch_domain_models::payments::PaymentIntentData<
                hyperswitch_domain_models::router_flow_types::payments::PaymentCreateIntent,
            >,
        >(
            state.clone(),
            req_state.clone(),
            merchant_account.clone(),
            profile.clone(),
            key_store.clone(),
            operations::PaymentIntentCreate,
            payload,
            global_payment_id,
            hyperswitch_domain_models::payments::HeaderPayload::default(),
            None,
        ))
        .await
        .change_context(errors::RevenueRecoveryError::PaymentIntentCreateFailed)?;
        router_env::logger::info!(?create_intent_response);
        let response = payments::handle_payments_intent_response(create_intent_response)
            .change_context(errors::RevenueRecoveryError::PaymentIntentCreateFailed)?;

        Ok(RecoveryPaymentIntent {
            payment_id: response.id,
            status: response.status,
            feature_metadata: response.feature_metadata,
        })
    }
}

impl RevenueRecoveryTransaction for RevenueRecoveryTransactionData {
    async fn get_payment_attempt(
        &self,
        state: &SessionState,
        req_state: &ReqState,
        merchant_account: &domain::MerchantAccount,
        profile: &domain::Profile,
        key_store: &domain::MerchantKeyStore,
        payment_id: common_utils::id_type::GlobalPaymentId,
    ) -> CustomResult<Option<RecoveryPaymentAttempt>, errors::RevenueRecoveryError> {
        let attempt_response = Box::pin(payments::payments_core::<
            hyperswitch_domain_models::router_flow_types::payments::PSync,
            api_models::payments::PaymentsRetrieveResponse,
            _,
            _,
            _,
            hyperswitch_domain_models::payments::PaymentStatusData<
                hyperswitch_domain_models::router_flow_types::payments::PSync,
            >,
        >(
            state.clone(),
            req_state.clone(),
            merchant_account.clone(),
            profile.clone(),
            key_store.clone(),
            operations::PaymentGet,
            api_models::payments::PaymentsRetrieveRequest {
                force_sync: false,
                expand_attempts: true,
                param: None,
            },
            payment_id.clone(),
            payments::CallConnectorAction::Avoid,
            hyperswitch_domain_models::payments::HeaderPayload::default(),
        ))
        .await;
        router_env::logger::info!(?attempt_response);
        let response = match attempt_response {
            Ok(services::ApplicationResponse::JsonWithHeaders((payments_response, _))) => {
                let final_attempt = payments_response.attempts.as_ref().and_then(|attempts| {
                    attempts.iter().find(|attempt| {
                        attempt.connector_payment_id.as_ref().is_some_and(|txn_id| {
                            Some(txn_id) == self.connector_transaction_id.as_ref()
                        })
                    })
                });
                let payment_attempt = final_attempt.map(|attempt_res| RecoveryPaymentAttempt {
                    attempt_id: attempt_res.id.to_owned(),
                    attempt_status: attempt_res.status.to_owned(),
                    feature_metadata: attempt_res.feature_metadata.to_owned(),
                });
                Ok(payment_attempt)
            }
            Ok(_) | Err(_) => Err(errors::RevenueRecoveryError::PaymentAttemptFetchFailed)
                .attach_printable("Failed to fetch Payment attempt in recovery webhook flow"),
        }?;
        Ok(response)
    }
    async fn record_payment_attempt(
        &self,
        _state: &SessionState,
        _req_state: &ReqState,
        _merchant_account: &domain::MerchantAccount,
        _profile: &domain::Profile,
        _key_store: &domain::MerchantKeyStore,
        _payment_id: common_utils::id_type::GlobalPaymentId,
    ) -> CustomResult<RecoveryPaymentAttempt, errors::RevenueRecoveryError> {
        todo!()
    }
}
