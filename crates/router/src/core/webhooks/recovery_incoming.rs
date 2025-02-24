use api_models::webhooks;
use common_utils::ext_traits::AsyncExt;
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::revenue_recovery;
use hyperswitch_interfaces::webhooks as interface_webhooks;
use router_env::{instrument, tracing};

use crate::{
    core::{
        errors::{self, CustomResult},
        payments::{self, operations},
    },
    routes::{app::ReqState, SessionState},
    services::{self, connector_integration_interface},
    types::{api, domain},
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
    request_details: &hyperswitch_interfaces::webhooks::IncomingWebhookRequestDetails<'_>,
    event_type: webhooks::IncomingWebhookEvent,
    req_state: ReqState,
) -> CustomResult<webhooks::WebhookResponseTracker, errors::RevenueRecoveryError> {
    // Source verification is necessary for revenue recovery webhooks flow since We don't have payment intent/attempt object created before in our system.

    common_utils::fp_utils::when(!source_verified, || {
        Err(report!(
            errors::RevenueRecoveryError::WebhookAuthenticationFailed
        ))
    })?;

    let invoice_details = RevenueRecoveryInvoice(
        interface_webhooks::IncomingWebhook::get_revenue_recovery_invoice_details(
            connector,
            request_details,
        )
        .change_context(errors::RevenueRecoveryError::InvoiceWebhookProcessingFailed)
        .attach_printable("Failed while getting revenue recovery invoice details")?,
    );
    // Fetch the intent using merchant reference id, if not found create new intent.
    let payment_intent = invoice_details
        .get_payment_intent(
            &state,
            &req_state,
            &merchant_account,
            &business_profile,
            &key_store,
        )
        .await
        .transpose()
        .async_unwrap_or_else(|| async {
            invoice_details
                .create_payment_intent(
                    &state,
                    &req_state,
                    &merchant_account,
                    &business_profile,
                    &key_store,
                )
                .await
        })
        .await?;

    let payment_attempt = match event_type.is_recovery_transaction_event() {
        true => {
            let invoice_transaction_details = RevenueRecoveryAttempt(
                interface_webhooks::IncomingWebhook::get_revenue_recovery_attempt_details(
                    connector,
                    request_details,
                )
                .change_context(errors::RevenueRecoveryError::TransactionWebhookProcessingFailed)?,
            );

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
        }
        false => None,
    };

    let attempt_triggered_by = payment_attempt
        .and_then(revenue_recovery::RecoveryPaymentAttempt::get_attempt_triggered_by);

    let action = revenue_recovery::RecoveryAction::get_action(event_type, attempt_triggered_by);

    match action {
        revenue_recovery::RecoveryAction::CancelInvoice => todo!(),
        revenue_recovery::RecoveryAction::ScheduleFailedPayment => {
            todo!()
        }
        revenue_recovery::RecoveryAction::SuccessPaymentExternal => {
            todo!()
        }
        revenue_recovery::RecoveryAction::PendingPayment => {
            router_env::logger::info!(
                "Pending transactions are not consumed by the revenue recovery webhooks"
            );
            Ok(webhooks::WebhookResponseTracker::NoEffect)
        }
        revenue_recovery::RecoveryAction::NoAction => {
            router_env::logger::info!(
                "No Recovery action is taken place for recovery event : {:?} and attempt triggered_by : {:?} ", event_type.clone(), attempt_triggered_by
            );
            Ok(webhooks::WebhookResponseTracker::NoEffect)
        }
        revenue_recovery::RecoveryAction::InvalidAction => {
            router_env::logger::error!(
                "Invalid Revenue recovery action state has been received, event : {:?}, triggered_by : {:?}", event_type, attempt_triggered_by
            );
            Ok(webhooks::WebhookResponseTracker::NoEffect)
        }
    }
}

pub struct RevenueRecoveryInvoice(revenue_recovery::RevenueRecoveryInvoiceData);
pub struct RevenueRecoveryAttempt(revenue_recovery::RevenueRecoveryAttemptData);

impl RevenueRecoveryInvoice {
    async fn get_payment_intent(
        &self,
        state: &SessionState,
        req_state: &ReqState,
        merchant_account: &domain::MerchantAccount,
        profile: &domain::Profile,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Option<revenue_recovery::RecoveryPaymentIntent>, errors::RevenueRecoveryError>
    {
        let payment_response = Box::pin(payments::payments_get_intent_using_merchant_reference(
            state.clone(),
            merchant_account.clone(),
            profile.clone(),
            key_store.clone(),
            req_state.clone(),
            &self.0.merchant_reference_id,
            hyperswitch_domain_models::payments::HeaderPayload::default(),
            None,
        ))
        .await;
        let response = match payment_response {
            Ok(services::ApplicationResponse::JsonWithHeaders((payments_response, _))) => {
                let payment_id = payments_response.id.clone();
                let status = payments_response.status;
                let feature_metadata = payments_response.feature_metadata;
                Ok(Some(revenue_recovery::RecoveryPaymentIntent {
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
            Ok(_) => Err(errors::RevenueRecoveryError::PaymentIntentFetchFailed)
                .attach_printable("Unexpected response from payment intent core"),
            error @ Err(_) => {
                router_env::logger::error!(?error);
                Err(errors::RevenueRecoveryError::PaymentIntentFetchFailed)
                    .attach_printable("failed to fetch payment intent recovery webhook flow")
            }
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
    ) -> CustomResult<revenue_recovery::RecoveryPaymentIntent, errors::RevenueRecoveryError> {
        let payload = api_models::payments::PaymentsCreateIntentRequest::from(&self.0);
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
            payments::operations::PaymentIntentCreate,
            payload,
            global_payment_id,
            hyperswitch_domain_models::payments::HeaderPayload::default(),
            None,
        ))
        .await
        .change_context(errors::RevenueRecoveryError::PaymentIntentCreateFailed)?;
        let response = payments::handle_payments_intent_response(create_intent_response)
            .change_context(errors::RevenueRecoveryError::PaymentIntentCreateFailed)?;

        Ok(revenue_recovery::RecoveryPaymentIntent {
            payment_id: response.id,
            status: response.status,
            feature_metadata: response.feature_metadata,
        })
    }
}

impl RevenueRecoveryAttempt {
    async fn get_payment_attempt(
        &self,
        state: &SessionState,
        req_state: &ReqState,
        merchant_account: &domain::MerchantAccount,
        profile: &domain::Profile,
        key_store: &domain::MerchantKeyStore,
        payment_id: common_utils::id_type::GlobalPaymentId,
    ) -> CustomResult<Option<revenue_recovery::RecoveryPaymentAttempt>, errors::RevenueRecoveryError>
    {
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
            payments::operations::PaymentGet,
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
        let response = match attempt_response {
            Ok(services::ApplicationResponse::JsonWithHeaders((payments_response, _))) => {
                let final_attempt =
                    self.0
                        .connector_transaction_id
                        .as_ref()
                        .and_then(|transaction_id| {
                            payments_response
                                .find_attempt_in_attempts_list_using_connector_transaction_id(
                                    transaction_id,
                                )
                        });
                let payment_attempt =
                    final_attempt.map(|attempt_res| revenue_recovery::RecoveryPaymentAttempt {
                        attempt_id: attempt_res.id.to_owned(),
                        attempt_status: attempt_res.status.to_owned(),
                        feature_metadata: attempt_res.feature_metadata.to_owned(),
                    });
                Ok(payment_attempt)
            }
            Ok(_) => Err(errors::RevenueRecoveryError::PaymentIntentFetchFailed)
                .attach_printable("Unexpected response from payment intent core"),
            error @ Err(_) => {
                router_env::logger::error!(?error);
                Err(errors::RevenueRecoveryError::PaymentIntentFetchFailed)
                    .attach_printable("failed to fetch payment intent recovery webhook flow")
            }
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
    ) -> CustomResult<revenue_recovery::RecoveryPaymentAttempt, errors::RevenueRecoveryError> {
        todo!()
    }
}
