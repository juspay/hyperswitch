use api_models::webhooks::{self, WebhookResponseTracker};
use common_utils::types::MinorUnit;
use error_stack::{report, ResultExt};
use hyperswitch_interfaces::webhooks::IncomingWebhookRequestDetails;
use router_env::{instrument, tracing};

use crate::core::payments::transformers::GenerateResponse;
use crate::{
    core::{
        api_locking,
        errors::{self, CustomResult},
        payments::{self, operations},
    },
    routes::{app::ReqState, lock_utils, SessionState},
    services::{self, connector_integration_interface},
    types::{
        api::{self},
        domain,
    },
};
use hyperswitch_interfaces::webhooks::IncomingWebhook;

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
#[cfg(feature = "recovery")]
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
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
    use error_stack::report;
    use hyperswitch_interfaces::recovery::{RecoveryAction, RecoveryActionTrait};

    match source_verified {
        true => {
            let invoice_details = connector
                .get_recovery_details(request_details)
                .change_context(errors::ApiErrorResponse::InternalServerError)?; // add attach_printable and add own errors.
                                                                                 // this should be fetched using merchant reference id api
            let payment_intent = invoice_details
                .get_intent(
                    state.clone(),
                    req_state.clone(),
                    merchant_account.clone(),
                    business_profile.clone(),
                    key_store.clone(),
                )
                .await?;
            let payment_attempt = invoice_details
                .get_attempt(
                    state,
                    req_state,
                    merchant_account,
                    business_profile,
                    key_store,
                    payment_intent.payment_id.clone(),
                )
                .await?;
            let triggered_by = payment_attempt.feature_metadata.and_then(|metadata| {
                metadata
                    .passive_churn_recovery
                    .map(|recovery| recovery.triggered_by)
            });

            let action = RecoveryAction::find_action(event_type, triggered_by);

            match action {
                RecoveryAction::CancelInvoice => todo!(),
                RecoveryAction::FailPaymentExternal => todo!(),
                RecoveryAction::SuccessPaymentExternal => todo!(),
                RecoveryAction::PendingPayment => todo!(),
                RecoveryAction::NoAction => todo!(),
                RecoveryAction::InvalidAction => todo!(),
            }
        }
        false => Err(report!(
            errors::ApiErrorResponse::WebhookAuthenticationFailed
        )),
    }
}

/// Trait definition
pub trait RecoveryTrait {
    /// Get the payment intent
    async fn get_intent(
        &self,
        state: SessionState,
        req_state: ReqState,
        merchant_account: domain::MerchantAccount,
        profile: domain::Profile,
        key_store: domain::MerchantKeyStore,
    ) -> CustomResult<RecoveryPaymentIntent, errors::ApiErrorResponse>;
    /// Get the payment attempt
    async fn get_attempt(
        &self,
        state: SessionState,
        req_state: ReqState,
        merchant_account: domain::MerchantAccount,
        profile: domain::Profile,
        key_store: domain::MerchantKeyStore,
        payment_id: common_utils::id_type::GlobalPaymentId,
    ) -> CustomResult<RecoveryPaymentAttempt, errors::ApiErrorResponse>;
    /// record attempt 
    async fn record_attempt(
        &self,
        state: SessionState,
        req_state: ReqState,
        merchant_account: domain::MerchantAccount,
        profile: domain::Profile,
        key_store: domain::MerchantKeyStore,
        payment_id: common_utils::id_type::GlobalPaymentId,
    ) -> CustomResult<RecoveryPaymentAttempt, errors::ApiErrorResponse>;
    // change this functions to from implmentations.
    fn create_payment_attempt_record_request(
        &self,
    ) -> api_models::payments::PaymentsAttemptRecordRequest;

    fn create_payment_intent_request(&self) -> api_models::payments::PaymentsCreateIntentRequest;

    fn create_payment_intent_amount_details(&self) -> api_models::payments::AmountDetails;
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

/// Implement the trait for RecoveryPayload
impl RecoveryTrait for hyperswitch_interfaces::recovery::RecoveryPayload {
    async fn get_intent(
        &self,
        state: SessionState,
        req_state: ReqState,
        merchant_account: domain::MerchantAccount,
        profile: domain::Profile,
        key_store: domain::MerchantKeyStore,
    ) -> CustomResult<RecoveryPaymentIntent, errors::ApiErrorResponse> {
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
        let response = match payment_response {
            Ok(value) => value,
            Err(err)
                if matches!(
                    err.current_context(),
                    &errors::ApiErrorResponse::PaymentNotFound
                ) =>
            {
                let payload = self.create_payment_intent_request();
                let global_payment_id = common_utils::id_type::GlobalPaymentId::generate(
                    &state.conf.cell_information.id.clone(),
                );
                Box::pin(payments::payments_intent_core::<
                    hyperswitch_domain_models::router_flow_types::PaymentCreateIntent,
                    api_models::payments::PaymentsIntentResponse,
                    _,
                    _,
                    hyperswitch_domain_models::payments::PaymentIntentData<
                        hyperswitch_domain_models::router_flow_types::PaymentCreateIntent,
                    >,
                >(
                    state.clone(),
                    req_state.clone(),
                    merchant_account.clone(),
                    profile.clone(),
                    key_store.clone(),
                    operations::PaymentIntentCreate,
                    payload,
                    global_payment_id.clone(),
                    hyperswitch_domain_models::payments::HeaderPayload::default(),
                    None,
                ))
                .await?
            }
            error @ Err(_) => error?,
        };
        match response {
            services::ApplicationResponse::JsonWithHeaders((payments_response, _)) => {
                let payment_id = payments_response.id.clone();
                let status = payments_response.status;
                let feature_metadata = payments_response.feature_metadata;
                Ok(RecoveryPaymentIntent {
                    payment_id,
                    status,
                    feature_metadata,
                })
            }
            _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure)
                .attach_printable("received non-json response from payments core")?,
        }
    }

    async fn get_attempt(
        &self,
        state: SessionState,
        req_state: ReqState,
        merchant_account: domain::MerchantAccount,
        profile: domain::Profile,
        key_store: domain::MerchantKeyStore,
        payment_id: common_utils::id_type::GlobalPaymentId,
    ) -> CustomResult<RecoveryPaymentAttempt, errors::ApiErrorResponse> {

        let key_manager_state = &(&state).into();
        let get_trackers_response =
            super::incoming_v2::get_trackers_response_for_payment_get_operation(
                state.store.as_ref(),
                &api_models::payments::PaymentIdType::PaymentIntentId(payment_id.clone()),
                profile.get_id(),
                key_manager_state,
                &key_store,
                merchant_account.storage_scheme,
            )
            .await?;

        let lock_action = api_locking::LockAction::Hold {
            input: api_locking::LockingInput {
                unique_locking_key: payment_id.get_string_repr().to_owned(),
                api_identifier: lock_utils::ApiIdentifier::Payments,
                override_lock_retries: None,
            },
        };

        lock_action
            .clone()
            .perform_locking_action(&state, merchant_account.get_id().to_owned())
            .await?;

        let (payment_data, _req, _, connector_http_status_code, external_latency) =
            Box::pin(payments::payments_operation_core::<
                api::PSync,
                _,
                _,
                _,
                hyperswitch_domain_models::payments::PaymentStatusData<api::PSync>,
            >(
                &state,
                req_state.clone(),
                merchant_account.clone(),
                key_store.clone(),
                profile.clone(),
                operations::PaymentGet,
                api::PaymentsRetrieveRequest {
                    force_sync: true,
                    expand_attempts: true,
                    param: None,
                },
                get_trackers_response,
                common_enums::enums::CallConnectorAction::Avoid,
                hyperswitch_domain_models::payments::HeaderPayload::default(),
            ))
            .await?;

        let attempt_response = GenerateResponse::generate_response(
            payment_data,
            &state,
            connector_http_status_code,
            external_latency,
            None,
            &merchant_account,
        )?;

        lock_action
            .free_lock_action(&state, merchant_account.get_id().to_owned())
            .await?;

        let attempt = match attempt_response {
            services::ApplicationResponse::JsonWithHeaders((payments_response, _)) => {
                let final_attempt = payments_response.attempts.as_ref().and_then(|attempts| {
                    attempts.iter().find(|attempt| {
                        attempt
                            .connector_payment_id
                            .as_ref()
                            .map_or(false, |txn_id| {
                                Some(txn_id) == self.connector_transaction_id.as_ref()
                            })
                    })
                });

                let payment_attempt = match final_attempt {
                    Some(attempt_res) => RecoveryPaymentAttempt {
                        attempt_id: attempt_res.id.to_owned(),
                        attempt_status: attempt_res.status.to_owned(),
                        feature_metadata: attempt_res.feature_metadata.to_owned(),
                    },
                    None => {
                        self.record_attempt(
                            state,
                            req_state,
                            merchant_account,
                            profile,
                            key_store,
                            payment_id.clone(),
                        )
                        .await?
                    }
                };
                payment_attempt
            }
            _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure)
                .attach_printable("received non-json response from payments core")?,
        };
        Ok(attempt)
    }

    async fn record_attempt(
        &self,
        state: SessionState,
        req_state: ReqState,
        merchant_account: domain::MerchantAccount,
        profile: domain::Profile,
        key_store: domain::MerchantKeyStore,
        payment_id: common_utils::id_type::GlobalPaymentId,
    ) -> CustomResult<RecoveryPaymentAttempt, errors::ApiErrorResponse> {
        let record_request = self.create_payment_attempt_record_request();
        let (payment_data, _) = Box::pin(payments::record_attempt_operation_core::<
            api::RecordAttempt,
            api_models::payments::PaymentsAttemptRecordRequest,
            api_models::payments::PaymentAttemptResponse,
            &operations::payment_attempt_record::PaymentAttemptRecord,
            hyperswitch_domain_models::payments::PaymentAttemptRecordData<api::RecordAttempt>,
        >(
            &state,
            req_state,
            merchant_account.clone(),
            profile,
            key_store.clone(),
            &operations::payment_attempt_record::PaymentAttemptRecord,
            record_request,
            payment_id,
            hyperswitch_domain_models::payments::HeaderPayload::default(),
            None,
        ))
        .await?;
        let attempt_response =
            payment_data.generate_response(&state, None, None, None, &merchant_account)?;
        match attempt_response {
            services::ApplicationResponse::JsonWithHeaders((attempt, _)) => {
                Ok(RecoveryPaymentAttempt {
                    attempt_id: attempt.id,
                    attempt_status: attempt.status,
                    feature_metadata: attempt.feature_metadata,
                })
            }
            _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure)
                .attach_printable("received non-json response from payments core")?,
        }
    }

    fn create_payment_intent_request(&self) -> api_models::payments::PaymentsCreateIntentRequest {
        let amount_details = self.create_payment_intent_amount_details();

        api_models::payments::PaymentsCreateIntentRequest {
            amount_details,
            merchant_reference_id: Some(self.merchant_reference_id.clone()),
            routing_algorithm_id: None,
            capture_method: Some(common_enums::CaptureMethod::Automatic),
            authentication_type: Some(common_enums::AuthenticationType::NoThreeDs),
            billing: None,
            shipping: None,
            customer_id: None,
            customer_present: Some(common_enums::PresenceOfCustomerDuringPayment::Absent),
            description: None,
            return_url: None,
            setup_future_usage: Some(common_enums::FutureUsage::OffSession),
            apply_mit_exemption: None,
            statement_descriptor: None,
            order_details: None,
            allowed_payment_method_types: None,
            metadata: None,
            connector_metadata: None,
            // This needs to update after payment intent db changes are merged in main
            feature_metadata: None,
            payment_link_enabled: None,
            payment_link_config: None,
            request_incremental_authorization: None,
            session_expiry: None,
            frm_metadata: None,
            request_external_three_ds_authentication: None,
        }
    }
    fn create_payment_intent_amount_details(&self) -> api_models::payments::AmountDetails {
        let amount = api_models::payments::AmountDetailsSetter {
            order_amount: self.amount.into(),
            currency: self.currency,
            shipping_cost: None,
            order_tax_amount: None,
            skip_external_tax_calculation: common_enums::TaxCalculationOverride::Skip,
            skip_surcharge_calculation: common_enums::SurchargeCalculationOverride::Skip,
            surcharge_amount: None,
            tax_on_surcharge: None,
        };
        api_models::payments::AmountDetails::new(amount)
    }
    fn create_payment_attempt_record_request(
        &self,
    ) -> api_models::payments::PaymentsAttemptRecordRequest {
        let amount_details = api_models::payments::PaymentAttemptAmountDetails {
            net_amount: self.amount,
            amount_to_capture: Some(MinorUnit::new(0)),
            surcharge_amount: None,
            tax_on_surcharge: None,
            amount_capturable: MinorUnit::new(0),
            shipping_cost: None,
            order_tax_amount: None,
        };
        api_models::payments::PaymentsAttemptRecordRequest {
            amount_details,
            status: self.status,
            billing: None,
            shipping: None,
            error_message: self.error_message.clone(),
            error_code: self.error_code.clone(),
            description: None,
            connector_transaction_id: self.connector_transaction_id.clone(),
            payment_method_type: self.payment_method_type,
            merchant_connector_reference_id: self.connector_account_reference_id.clone(),
            payment_method_subtype: self.payment_method_sub_type,
            payment_method_data: None,
            metadata: None,
            feature_metadata: None,
            created_at: self.created_at,
        }
    }
}
