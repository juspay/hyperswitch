use std::{collections::HashMap, marker::PhantomData, str::FromStr};

use api_models::{enums as api_enums, payments as api_payments, webhooks};
use common_utils::{
    ext_traits::{AsyncExt, ValueExt},
    id_type,
};
use diesel_models::process_tracker as storage;
use error_stack::{report, ResultExt};
use futures::stream::SelectNextSome;
use hyperswitch_domain_models::{
    payments as domain_payments,
    revenue_recovery::{self, RecoveryPaymentIntent},
    router_data_v2::flow_common_types,
    router_flow_types,
    router_request_types::revenue_recovery as revenue_recovery_request,
    router_response_types::revenue_recovery as revenue_recovery_response,
    types as router_types,
};
use hyperswitch_interfaces::webhooks as interface_webhooks;
use masking::{PeekInterface, Secret};
use router_env::{instrument, logger, tracing};
use services::kafka;
use storage::business_status;

use crate::{
    core::{
        self, admin,
        errors::{self, CustomResult},
        payments::{self, helpers},
    },
    db::{errors::RevenueRecoveryError, StorageInterface},
    routes::{app::ReqState, metrics, SessionState},
    services::{
        self,
        connector_integration_interface::{self, RouterDataConversion},
    },
    types::{
        self, api, domain,
        storage::{
            revenue_recovery as storage_revenue_recovery,
            revenue_recovery_redis_operation::{
                PaymentProcessorTokenDetails, PaymentProcessorTokenStatus, RedisTokenManager,
            },
        },
        transformers::ForeignFrom,
    },
    workflows::revenue_recovery as revenue_recovery_flow,
};
#[cfg(feature = "v2")]
pub const REVENUE_RECOVERY: &str = "revenue_recovery";

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
#[cfg(feature = "revenue_recovery")]
pub async fn recovery_incoming_webhook_flow(
    state: SessionState,
    platform: domain::Platform,
    business_profile: domain::Profile,
    source_verified: bool,
    connector_enum: &connector_integration_interface::ConnectorEnum,
    billing_connector_account: hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
    connector_name: &str,
    request_details: &hyperswitch_interfaces::webhooks::IncomingWebhookRequestDetails<'_>,
    event_type: webhooks::IncomingWebhookEvent,
    req_state: ReqState,
    object_ref_id: &webhooks::ObjectReferenceId,
) -> CustomResult<webhooks::WebhookResponseTracker, errors::RevenueRecoveryError> {
    // Source verification is necessary for revenue recovery webhooks flow since We don't have payment intent/attempt object created before in our system.
    common_utils::fp_utils::when(!source_verified, || {
        Err(report!(
            errors::RevenueRecoveryError::WebhookAuthenticationFailed
        ))
    })?;

    let connector = api_enums::Connector::from_str(connector_name)
        .change_context(errors::RevenueRecoveryError::InvoiceWebhookProcessingFailed)
        .attach_printable_lazy(|| format!("unable to parse connector name {connector_name:?}"))?;

    let billing_connectors_with_invoice_sync_call = &state.conf.billing_connectors_invoice_sync;

    let should_billing_connector_invoice_api_called = billing_connectors_with_invoice_sync_call
        .billing_connectors_which_requires_invoice_sync_call
        .contains(&connector);

    let billing_connectors_with_payment_sync_call = &state.conf.billing_connectors_payment_sync;

    let should_billing_connector_payment_api_called = billing_connectors_with_payment_sync_call
        .billing_connectors_which_require_payment_sync
        .contains(&connector);

    let billing_connector_payment_details =
        BillingConnectorPaymentsSyncResponseData::get_billing_connector_payment_details(
            should_billing_connector_payment_api_called,
            &state,
            &platform,
            &billing_connector_account,
            connector_name,
            object_ref_id,
        )
        .await?;

    let invoice_id = billing_connector_payment_details
        .clone()
        .map(|data| data.merchant_reference_id);

    let billing_connector_invoice_details =
        BillingConnectorInvoiceSyncResponseData::get_billing_connector_invoice_details(
            should_billing_connector_invoice_api_called,
            &state,
            &platform,
            &billing_connector_account,
            connector_name,
            invoice_id,
        )
        .await?;

    // Checks whether we have data in billing_connector_invoice_details , if it is there then we construct revenue recovery invoice from it else it takes from webhook
    let invoice_details = RevenueRecoveryInvoice::get_recovery_invoice_details(
        connector_enum,
        request_details,
        billing_connector_invoice_details.as_ref(),
    )?;

    // Fetch the intent using merchant reference id, if not found create new intent.
    let payment_intent = invoice_details
        .get_payment_intent(&state, &req_state, &platform, &business_profile)
        .await
        .transpose()
        .async_unwrap_or_else(|| async {
            invoice_details
                .create_payment_intent(&state, &req_state, &platform, &business_profile)
                .await
        })
        .await?;

    let is_event_recovery_transaction_event = event_type.is_recovery_transaction_event();
    let (recovery_attempt_from_payment_attempt, recovery_intent_from_payment_attempt) =
        RevenueRecoveryAttempt::get_recovery_payment_attempt(
            is_event_recovery_transaction_event,
            &billing_connector_account,
            &state,
            connector_enum,
            &req_state,
            billing_connector_payment_details.as_ref(),
            request_details,
            &platform,
            &business_profile,
            &payment_intent,
            &invoice_details.0,
        )
        .await?;

    // Publish event to Kafka
    if let Some(ref attempt) = recovery_attempt_from_payment_attempt {
        // Passing `platform` here
        let recovery_payment_tuple =
            &RecoveryPaymentTuple::new(&recovery_intent_from_payment_attempt, attempt);
        if let Err(e) = RecoveryPaymentTuple::publish_revenue_recovery_event_to_kafka(
            &state,
            recovery_payment_tuple,
            None,
        )
        .await
        {
            logger::error!(
                "Failed to publish revenue recovery event to kafka : {:?}",
                e
            );
        };
    }

    let attempt_triggered_by = recovery_attempt_from_payment_attempt
        .as_ref()
        .and_then(|attempt| attempt.get_attempt_triggered_by());

    let recovery_action = RecoveryAction {
        action: RecoveryAction::get_action(event_type, attempt_triggered_by),
    };

    let mca_retry_threshold = billing_connector_account
        .get_retry_threshold()
        .ok_or(report!(
            errors::RevenueRecoveryError::BillingThresholdRetryCountFetchFailed
        ))?;

    let intent_retry_count = recovery_intent_from_payment_attempt
        .feature_metadata
        .as_ref()
        .and_then(|metadata| metadata.get_retry_count())
        .ok_or(report!(errors::RevenueRecoveryError::RetryCountFetchFailed))?;

    logger::info!("Intent retry count: {:?}", intent_retry_count);
    recovery_action
        .handle_action(
            &state,
            &business_profile,
            &platform,
            &billing_connector_account,
            mca_retry_threshold,
            intent_retry_count,
            &(
                recovery_attempt_from_payment_attempt,
                recovery_intent_from_payment_attempt,
            ),
        )
        .await
}

async fn handle_monitoring_threshold(
    state: &SessionState,
    business_profile: &domain::Profile,
    key_store: &domain::MerchantKeyStore,
) -> CustomResult<webhooks::WebhookResponseTracker, errors::RevenueRecoveryError> {
    let db = &*state.store;
    let key_manager_state = &(state).into();
    let monitoring_threshold_config = state.conf.revenue_recovery.monitoring_threshold_in_seconds;
    let retry_algorithm_type = state.conf.revenue_recovery.retry_algorithm_type;
    let revenue_recovery_retry_algorithm = business_profile
        .revenue_recovery_retry_algorithm_data
        .clone()
        .ok_or(report!(
            errors::RevenueRecoveryError::RetryAlgorithmTypeNotFound
        ))?;
    if revenue_recovery_retry_algorithm
        .has_exceeded_monitoring_threshold(monitoring_threshold_config)
    {
        let profile_wrapper = admin::ProfileWrapper::new(business_profile.clone());
        profile_wrapper
            .update_revenue_recovery_algorithm_under_profile(
                db,
                key_manager_state,
                key_store,
                retry_algorithm_type,
            )
            .await
            .change_context(errors::RevenueRecoveryError::RetryAlgorithmUpdationFailed)?;
    }
    Ok(webhooks::WebhookResponseTracker::NoEffect)
}

#[allow(clippy::too_many_arguments)]
async fn handle_schedule_failed_payment(
    billing_connector_account: &domain::MerchantConnectorAccount,
    intent_retry_count: u16,
    mca_retry_threshold: u16,
    state: &SessionState,
    platform: &domain::Platform,
    payment_attempt_with_recovery_intent: &(
        Option<revenue_recovery::RecoveryPaymentAttempt>,
        revenue_recovery::RecoveryPaymentIntent,
    ),
    business_profile: &domain::Profile,
    revenue_recovery_retry: api_enums::RevenueRecoveryAlgorithmType,
) -> CustomResult<webhooks::WebhookResponseTracker, errors::RevenueRecoveryError> {
    let (recovery_attempt_from_payment_attempt, recovery_intent_from_payment_attempt) =
        payment_attempt_with_recovery_intent;

    // When intent_retry_count is less than or equal to threshold
    (intent_retry_count <= mca_retry_threshold)
        .then(|| {
            logger::error!(
                "Payment retry count {} is less than threshold {}",
                intent_retry_count,
                mca_retry_threshold
            );
            Ok(webhooks::WebhookResponseTracker::NoEffect)
        })
        .async_unwrap_or_else(|| async {
            // Call calculate_job
            core::revenue_recovery::upsert_calculate_pcr_task(
                billing_connector_account,
                state,
                platform,
                recovery_intent_from_payment_attempt,
                business_profile,
                intent_retry_count,
                recovery_attempt_from_payment_attempt
                    .as_ref()
                    .map(|attempt| attempt.attempt_id.clone()),
                storage::ProcessTrackerRunner::PassiveRecoveryWorkflow,
                revenue_recovery_retry,
            )
            .await
        })
        .await
}

#[derive(Debug)]
pub struct RevenueRecoveryInvoice(revenue_recovery::RevenueRecoveryInvoiceData);
#[derive(Debug)]
pub struct RevenueRecoveryAttempt(revenue_recovery::RevenueRecoveryAttemptData);

impl RevenueRecoveryInvoice {
    pub async fn get_or_create_custom_recovery_intent(
        data: api_models::payments::RecoveryPaymentsCreate,
        state: &SessionState,
        req_state: &ReqState,
        platform: &domain::Platform,
        profile: &domain::Profile,
    ) -> CustomResult<revenue_recovery::RecoveryPaymentIntent, errors::RevenueRecoveryError> {
        let recovery_intent = Self(revenue_recovery::RevenueRecoveryInvoiceData::foreign_from(
            data,
        ));
        recovery_intent
            .get_payment_intent(state, req_state, platform, profile)
            .await
            .transpose()
            .async_unwrap_or_else(|| async {
                recovery_intent
                    .create_payment_intent(state, req_state, platform, profile)
                    .await
            })
            .await
    }
    fn get_recovery_invoice_details(
        connector_enum: &connector_integration_interface::ConnectorEnum,
        request_details: &hyperswitch_interfaces::webhooks::IncomingWebhookRequestDetails<'_>,
        billing_connector_invoice_details: Option<
            &revenue_recovery_response::BillingConnectorInvoiceSyncResponse,
        >,
    ) -> CustomResult<Self, errors::RevenueRecoveryError> {
        billing_connector_invoice_details.map_or_else(
            || {
                interface_webhooks::IncomingWebhook::get_revenue_recovery_invoice_details(
                    connector_enum,
                    request_details,
                )
                .change_context(errors::RevenueRecoveryError::InvoiceWebhookProcessingFailed)
                .attach_printable("Failed while getting revenue recovery invoice details")
                .map(RevenueRecoveryInvoice)
            },
            |data| {
                Ok(Self(revenue_recovery::RevenueRecoveryInvoiceData::from(
                    data,
                )))
            },
        )
    }

    async fn get_payment_intent(
        &self,
        state: &SessionState,
        req_state: &ReqState,
        platform: &domain::Platform,
        profile: &domain::Profile,
    ) -> CustomResult<Option<revenue_recovery::RecoveryPaymentIntent>, errors::RevenueRecoveryError>
    {
        let payment_response = Box::pin(payments::payments_get_intent_using_merchant_reference(
            state.clone(),
            platform.clone(),
            profile.clone(),
            req_state.clone(),
            &self.0.merchant_reference_id,
            hyperswitch_domain_models::payments::HeaderPayload::default(),
        ))
        .await;
        let response = match payment_response {
            Ok(services::ApplicationResponse::JsonWithHeaders((payments_response, _))) => {
                let payment_id = payments_response.id.clone();
                let status = payments_response.status;
                let feature_metadata = payments_response.feature_metadata;
                let merchant_id = platform.get_processor().get_account().get_id().clone();
                let revenue_recovery_invoice_data = &self.0;
                Ok(Some(revenue_recovery::RecoveryPaymentIntent {
                    payment_id,
                    status,
                    feature_metadata,
                    merchant_id,
                    merchant_reference_id: Some(
                        revenue_recovery_invoice_data.merchant_reference_id.clone(),
                    ),
                    invoice_amount: revenue_recovery_invoice_data.amount,
                    invoice_currency: revenue_recovery_invoice_data.currency,
                    created_at: revenue_recovery_invoice_data.billing_started_at,
                    billing_address: revenue_recovery_invoice_data.billing_address.clone(),
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
                logger::error!(?error);
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
        platform: &domain::Platform,
        profile: &domain::Profile,
    ) -> CustomResult<revenue_recovery::RecoveryPaymentIntent, errors::RevenueRecoveryError> {
        let payload = api_payments::PaymentsCreateIntentRequest::from(&self.0);
        let global_payment_id = id_type::GlobalPaymentId::generate(&state.conf.cell_information.id);

        let create_intent_response = Box::pin(payments::payments_intent_core::<
            router_flow_types::payments::PaymentCreateIntent,
            api_payments::PaymentsIntentResponse,
            _,
            _,
            hyperswitch_domain_models::payments::PaymentIntentData<
                router_flow_types::payments::PaymentCreateIntent,
            >,
        >(
            state.clone(),
            req_state.clone(),
            platform.clone(),
            profile.clone(),
            payments::operations::PaymentIntentCreate,
            payload,
            global_payment_id,
            hyperswitch_domain_models::payments::HeaderPayload::default(),
        ))
        .await
        .change_context(errors::RevenueRecoveryError::PaymentIntentCreateFailed)?;

        let response = create_intent_response
            .get_json_body()
            .change_context(errors::RevenueRecoveryError::PaymentIntentCreateFailed)
            .attach_printable("expected json response")?;

        let merchant_id = platform.get_processor().get_account().get_id().clone();
        let revenue_recovery_invoice_data = &self.0;

        Ok(revenue_recovery::RecoveryPaymentIntent {
            payment_id: response.id,
            status: response.status,
            feature_metadata: response.feature_metadata,
            merchant_id,
            merchant_reference_id: Some(
                revenue_recovery_invoice_data.merchant_reference_id.clone(),
            ),
            invoice_amount: revenue_recovery_invoice_data.amount,
            invoice_currency: revenue_recovery_invoice_data.currency,
            created_at: revenue_recovery_invoice_data.billing_started_at,
            billing_address: revenue_recovery_invoice_data.billing_address.clone(),
        })
    }
}

impl RevenueRecoveryAttempt {
    pub async fn load_recovery_attempt_from_api(
        data: api_models::payments::RecoveryPaymentsCreate,
        state: &SessionState,
        req_state: &ReqState,
        platform: &domain::Platform,
        profile: &domain::Profile,
        payment_intent: revenue_recovery::RecoveryPaymentIntent,
        payment_merchant_connector_account: domain::MerchantConnectorAccount,
    ) -> CustomResult<
        (
            revenue_recovery::RecoveryPaymentAttempt,
            revenue_recovery::RecoveryPaymentIntent,
        ),
        errors::RevenueRecoveryError,
    > {
        let recovery_attempt = Self(revenue_recovery::RevenueRecoveryAttemptData::foreign_from(
            &data,
        ));
        recovery_attempt
            .get_payment_attempt(state, req_state, platform, profile, &payment_intent)
            .await
            .transpose()
            .async_unwrap_or_else(|| async {
                recovery_attempt
                    .record_payment_attempt(
                        state,
                        req_state,
                        platform,
                        profile,
                        &payment_intent,
                        &data.billing_merchant_connector_id,
                        Some(payment_merchant_connector_account),
                    )
                    .await
            })
            .await
    }

    fn get_recovery_invoice_transaction_details(
        connector_enum: &connector_integration_interface::ConnectorEnum,
        request_details: &hyperswitch_interfaces::webhooks::IncomingWebhookRequestDetails<'_>,
        billing_connector_payment_details: Option<
            &revenue_recovery_response::BillingConnectorPaymentsSyncResponse,
        >,
        billing_connector_invoice_details: &revenue_recovery::RevenueRecoveryInvoiceData,
    ) -> CustomResult<Self, errors::RevenueRecoveryError> {
        billing_connector_payment_details.map_or_else(
            || {
                interface_webhooks::IncomingWebhook::get_revenue_recovery_attempt_details(
                    connector_enum,
                    request_details,
                )
                .change_context(errors::RevenueRecoveryError::TransactionWebhookProcessingFailed)
                .attach_printable(
                    "Failed to get recovery attempt details from the billing connector",
                )
                .map(RevenueRecoveryAttempt)
            },
            |data| {
                Ok(Self(revenue_recovery::RevenueRecoveryAttemptData::from((
                    data,
                    billing_connector_invoice_details,
                ))))
            },
        )
    }
    pub fn get_revenue_recovery_attempt(
        payment_intent: &domain_payments::PaymentIntent,
        revenue_recovery_metadata: &api_payments::PaymentRevenueRecoveryMetadata,
        billing_connector_account: &domain::MerchantConnectorAccount,
        card_info: api_payments::AdditionalCardInfo,
        payment_processor_token: &str,
    ) -> CustomResult<Self, errors::RevenueRecoveryError> {
        let revenue_recovery_data = payment_intent
            .create_revenue_recovery_attempt_data(
                revenue_recovery_metadata.clone(),
                billing_connector_account,
                card_info,
                payment_processor_token,
            )
            .change_context(errors::RevenueRecoveryError::RevenueRecoveryAttemptDataCreateFailed)
            .attach_printable("Failed to build recovery attempt data")?;
        Ok(Self(revenue_recovery_data))
    }
    async fn get_payment_attempt(
        &self,
        state: &SessionState,
        req_state: &ReqState,
        platform: &domain::Platform,
        profile: &domain::Profile,
        payment_intent: &revenue_recovery::RecoveryPaymentIntent,
    ) -> CustomResult<
        Option<(
            revenue_recovery::RecoveryPaymentAttempt,
            revenue_recovery::RecoveryPaymentIntent,
        )>,
        errors::RevenueRecoveryError,
    > {
        let attempt_response =
            Box::pin(payments::payments_list_attempts_using_payment_intent_id::<
                payments::operations::PaymentGetListAttempts,
                api_payments::PaymentAttemptListResponse,
                _,
                payments::operations::payment_attempt_list::PaymentGetListAttempts,
                hyperswitch_domain_models::payments::PaymentAttemptListData<
                    payments::operations::PaymentGetListAttempts,
                >,
            >(
                state.clone(),
                req_state.clone(),
                platform.clone(),
                profile.clone(),
                payments::operations::PaymentGetListAttempts,
                api_payments::PaymentAttemptListRequest {
                    payment_intent_id: payment_intent.payment_id.clone(),
                },
                payment_intent.payment_id.clone(),
                hyperswitch_domain_models::payments::HeaderPayload::default(),
            ))
            .await;
        let response = match attempt_response {
            Ok(services::ApplicationResponse::JsonWithHeaders((payments_response, _))) => {
                let final_attempt = self
                    .0
                    .charge_id
                    .as_ref()
                    .map(|charge_id| {
                        payments_response
                            .find_attempt_in_attempts_list_using_charge_id(charge_id.clone())
                    })
                    .unwrap_or_else(|| {
                        self.0
                            .connector_transaction_id
                            .as_ref()
                            .and_then(|transaction_id| {
                                payments_response
                                    .find_attempt_in_attempts_list_using_connector_transaction_id(
                                        transaction_id,
                                    )
                            })
                    });
                let payment_attempt =
                    final_attempt.map(|res| revenue_recovery::RecoveryPaymentAttempt {
                        attempt_id: res.id.to_owned(),
                        attempt_status: res.status.to_owned(),
                        feature_metadata: res.feature_metadata.to_owned(),
                        amount: res.amount.net_amount,
                        network_advice_code: res.error.clone().and_then(|e| e.network_advice_code), // Placeholder, to be populated if available
                        network_decline_code: res
                            .error
                            .clone()
                            .and_then(|e| e.network_decline_code), // Placeholder, to be populated if available
                        error_code: res.error.clone().map(|error| error.code),
                        created_at: res.created_at,
                    });
                // If we have an attempt, combine it with payment_intent in a tuple.
                let res_with_payment_intent_and_attempt =
                    payment_attempt.map(|attempt| (attempt, (*payment_intent).clone()));
                Ok(res_with_payment_intent_and_attempt)
            }
            Ok(_) => Err(errors::RevenueRecoveryError::PaymentAttemptFetchFailed)
                .attach_printable("Unexpected response from payment intent core"),
            error @ Err(_) => {
                logger::error!(?error);
                Err(errors::RevenueRecoveryError::PaymentAttemptFetchFailed)
                    .attach_printable("failed to fetch payment attempt in recovery webhook flow")
            }
        }?;
        Ok(response)
    }

    #[allow(clippy::too_many_arguments)]
    async fn record_payment_attempt(
        &self,
        state: &SessionState,
        req_state: &ReqState,
        platform: &domain::Platform,
        profile: &domain::Profile,
        payment_intent: &revenue_recovery::RecoveryPaymentIntent,
        billing_connector_account_id: &id_type::MerchantConnectorAccountId,
        payment_connector_account: Option<domain::MerchantConnectorAccount>,
    ) -> CustomResult<
        (
            revenue_recovery::RecoveryPaymentAttempt,
            revenue_recovery::RecoveryPaymentIntent,
        ),
        errors::RevenueRecoveryError,
    > {
        let payment_connector_id =   payment_connector_account.as_ref().map(|account: &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount| account.id.clone());
        let payment_connector_name = payment_connector_account
            .as_ref()
            .map(|account| account.connector_name);
        let request_payload: api_payments::PaymentsAttemptRecordRequest = self
            .create_payment_record_request(
                state,
                billing_connector_account_id,
                payment_connector_id,
                payment_connector_name,
                common_enums::TriggeredBy::External,
            )
            .await?;
        let attempt_response = Box::pin(payments::record_attempt_core(
            state.clone(),
            req_state.clone(),
            platform.clone(),
            profile.clone(),
            request_payload,
            payment_intent.payment_id.clone(),
            hyperswitch_domain_models::payments::HeaderPayload::default(),
        ))
        .await;

        let (recovery_attempt, updated_recovery_intent) = match attempt_response {
            Ok(services::ApplicationResponse::JsonWithHeaders((attempt_response, _))) => {
                Ok((
                    revenue_recovery::RecoveryPaymentAttempt {
                        attempt_id: attempt_response.id.clone(),
                        attempt_status: attempt_response.status,
                        feature_metadata: attempt_response.payment_attempt_feature_metadata,
                        amount: attempt_response.amount,
                        network_advice_code: attempt_response
                            .error_details
                            .clone()
                            .and_then(|error| error.network_decline_code), // Placeholder, to be populated if available
                        network_decline_code: attempt_response
                            .error_details
                            .clone()
                            .and_then(|error| error.network_decline_code), // Placeholder, to be populated if available
                        error_code: attempt_response
                            .error_details
                            .clone()
                            .map(|error| error.code),
                        created_at: attempt_response.created_at,
                    },
                    revenue_recovery::RecoveryPaymentIntent {
                        payment_id: payment_intent.payment_id.clone(),
                        status: attempt_response.status.into(), // Using status from attempt_response
                        feature_metadata: attempt_response.payment_intent_feature_metadata, // Using feature_metadata from attempt_response
                        merchant_id: payment_intent.merchant_id.clone(),
                        merchant_reference_id: payment_intent.merchant_reference_id.clone(),
                        invoice_amount: payment_intent.invoice_amount,
                        invoice_currency: payment_intent.invoice_currency,
                        created_at: payment_intent.created_at,
                        billing_address: payment_intent.billing_address.clone(),
                    },
                ))
            }
            Ok(_) => Err(errors::RevenueRecoveryError::PaymentAttemptFetchFailed)
                .attach_printable("Unexpected response from record attempt core"),
            error @ Err(_) => {
                logger::error!(?error);
                Err(errors::RevenueRecoveryError::PaymentAttemptFetchFailed)
                    .attach_printable("failed to record attempt in recovery webhook flow")
            }
        }?;

        let response = (recovery_attempt, updated_recovery_intent);

        self.store_payment_processor_tokens_in_redis(state, &response.0, payment_connector_name)
            .await
            .map_err(|e| {
                router_env::logger::error!(
                    "Failed to store payment processor tokens in Redis: {:?}",
                    e
                );
                errors::RevenueRecoveryError::RevenueRecoveryRedisInsertFailed
            })?;

        Ok(response)
    }

    pub async fn create_payment_record_request(
        &self,
        state: &SessionState,
        billing_merchant_connector_account_id: &id_type::MerchantConnectorAccountId,
        payment_merchant_connector_account_id: Option<id_type::MerchantConnectorAccountId>,
        payment_connector: Option<common_enums::connector_enums::Connector>,
        triggered_by: common_enums::TriggeredBy,
    ) -> CustomResult<api_payments::PaymentsAttemptRecordRequest, errors::RevenueRecoveryError>
    {
        let revenue_recovery_attempt_data = &self.0;
        let amount_details =
            api_payments::PaymentAttemptAmountDetails::from(revenue_recovery_attempt_data);
        let feature_metadata = api_payments::PaymentAttemptFeatureMetadata {
            revenue_recovery: Some(api_payments::PaymentAttemptRevenueRecoveryData {
                // Since we are recording the external paymenmt attempt, this is hardcoded to External
                attempt_triggered_by: triggered_by,
                charge_id: self.0.charge_id.clone(),
            }),
        };

        let card_info = revenue_recovery_attempt_data
            .card_info
            .card_isin
            .clone()
            .async_and_then(|isin| async move {
                let issuer_identifier_number = isin.clone();
                state
                    .store
                    .get_card_info(issuer_identifier_number.as_str())
                    .await
                    .map_err(|error| services::logger::warn!(card_info_error=?error))
                    .ok()
            })
            .await
            .flatten();
        let payment_method_data = api_models::payments::RecordAttemptPaymentMethodDataRequest {
            payment_method_data: api_models::payments::AdditionalPaymentData::Card(Box::new(
                revenue_recovery_attempt_data.card_info.clone(),
            )),
            billing: None,
        };

        let card_issuer = revenue_recovery_attempt_data.card_info.card_issuer.clone();

        let error =
            Option::<api_payments::RecordAttemptErrorDetails>::from(revenue_recovery_attempt_data);
        Ok(api_payments::PaymentsAttemptRecordRequest {
            amount_details,
            status: revenue_recovery_attempt_data.status,
            billing: None,
            shipping: None,
            connector: payment_connector,
            payment_merchant_connector_id: payment_merchant_connector_account_id,
            error,
            description: None,
            connector_transaction_id: revenue_recovery_attempt_data
                .connector_transaction_id
                .clone(),
            payment_method_type: revenue_recovery_attempt_data.payment_method_type,
            billing_connector_id: billing_merchant_connector_account_id.clone(),
            payment_method_subtype: revenue_recovery_attempt_data.payment_method_sub_type,
            payment_method_data: Some(payment_method_data),
            metadata: None,
            feature_metadata: Some(feature_metadata),
            transaction_created_at: revenue_recovery_attempt_data.transaction_created_at,
            processor_payment_method_token: revenue_recovery_attempt_data
                .processor_payment_method_token
                .clone(),
            connector_customer_id: revenue_recovery_attempt_data.connector_customer_id.clone(),
            retry_count: revenue_recovery_attempt_data.retry_count,
            invoice_next_billing_time: revenue_recovery_attempt_data.invoice_next_billing_time,
            invoice_billing_started_at_time: revenue_recovery_attempt_data
                .invoice_billing_started_at_time,
            triggered_by,
            card_network: revenue_recovery_attempt_data.card_info.card_network.clone(),
            card_issuer,
        })
    }

    pub async fn find_payment_merchant_connector_account(
        &self,
        state: &SessionState,
        key_store: &domain::MerchantKeyStore,
        billing_connector_account: &domain::MerchantConnectorAccount,
    ) -> CustomResult<Option<domain::MerchantConnectorAccount>, errors::RevenueRecoveryError> {
        let payment_merchant_connector_account_id = billing_connector_account
            .get_payment_merchant_connector_account_id_using_account_reference_id(
                self.0.connector_account_reference_id.clone(),
            );
        let db = &*state.store;
        let payment_merchant_connector_account = payment_merchant_connector_account_id
            .as_ref()
            .async_map(|mca_id| async move {
                db.find_merchant_connector_account_by_id(mca_id, key_store)
                    .await
            })
            .await
            .transpose()
            .change_context(errors::RevenueRecoveryError::PaymentMerchantConnectorAccountNotFound)
            .attach_printable(
                "failed to fetch payment merchant connector id using account reference id",
            )?;
        Ok(payment_merchant_connector_account)
    }

    #[allow(clippy::too_many_arguments)]
    async fn get_recovery_payment_attempt(
        is_recovery_transaction_event: bool,
        billing_connector_account: &domain::MerchantConnectorAccount,
        state: &SessionState,
        connector_enum: &connector_integration_interface::ConnectorEnum,
        req_state: &ReqState,
        billing_connector_payment_details: Option<
            &revenue_recovery_response::BillingConnectorPaymentsSyncResponse,
        >,
        request_details: &hyperswitch_interfaces::webhooks::IncomingWebhookRequestDetails<'_>,
        platform: &domain::Platform,
        business_profile: &domain::Profile,
        payment_intent: &revenue_recovery::RecoveryPaymentIntent,
        invoice_details: &revenue_recovery::RevenueRecoveryInvoiceData,
    ) -> CustomResult<
        (
            Option<revenue_recovery::RecoveryPaymentAttempt>,
            revenue_recovery::RecoveryPaymentIntent,
        ),
        errors::RevenueRecoveryError,
    > {
        let payment_attempt_with_recovery_intent = match is_recovery_transaction_event {
            true => {
                let invoice_transaction_details = Self::get_recovery_invoice_transaction_details(
                    connector_enum,
                    request_details,
                    billing_connector_payment_details,
                    invoice_details,
                )?;

                // Find the payment merchant connector ID at the top level to avoid multiple DB calls.
                let payment_merchant_connector_account = invoice_transaction_details
                    .find_payment_merchant_connector_account(
                        state,
                        platform.get_processor().get_key_store(),
                        billing_connector_account,
                    )
                    .await?;

                let (payment_attempt, updated_payment_intent) = invoice_transaction_details
                    .get_payment_attempt(
                        state,
                        req_state,
                        platform,
                        business_profile,
                        payment_intent,
                    )
                    .await
                    .transpose()
                    .async_unwrap_or_else(|| async {
                        invoice_transaction_details
                            .record_payment_attempt(
                                state,
                                req_state,
                                platform,
                                business_profile,
                                payment_intent,
                                &billing_connector_account.get_id(),
                                payment_merchant_connector_account,
                            )
                            .await
                    })
                    .await?;
                (Some(payment_attempt), updated_payment_intent)
            }

            false => (None, payment_intent.clone()),
        };

        Ok(payment_attempt_with_recovery_intent)
    }

    /// Store payment processor tokens in Redis for retry management
    async fn store_payment_processor_tokens_in_redis(
        &self,
        state: &SessionState,
        recovery_attempt: &revenue_recovery::RecoveryPaymentAttempt,
        payment_connector_name: Option<common_enums::connector_enums::Connector>,
    ) -> CustomResult<(), errors::RevenueRecoveryError> {
        let revenue_recovery_attempt_data = &self.0;

        let error_code = revenue_recovery_attempt_data.error_code.clone();
        let error_message = revenue_recovery_attempt_data.error_message.clone();
        let connector_name = payment_connector_name
            .ok_or(errors::RevenueRecoveryError::TransactionWebhookProcessingFailed)
            .attach_printable("unable to derive payment connector")?
            .to_string();

        let gsm_record = helpers::get_gsm_record(
            state,
            error_code.clone(),
            error_message,
            connector_name,
            REVENUE_RECOVERY.to_string(),
        )
        .await;

        let is_hard_decline = gsm_record
            .and_then(|record| record.error_category)
            .map(|category| category == common_enums::ErrorCategory::HardDecline)
            .unwrap_or(false);

        let reference_time = time::PrimitiveDateTime::new(
            recovery_attempt.created_at.date(),
            time::Time::from_hms(recovery_attempt.created_at.hour(), 0, 0)
                .unwrap_or(time::Time::MIDNIGHT),
        );

        // Extract required fields from the revenue recovery attempt data
        let connector_customer_id = revenue_recovery_attempt_data.connector_customer_id.clone();

        let attempt_id = recovery_attempt.attempt_id.clone();
        let token_unit = PaymentProcessorTokenStatus {
            error_code,
            inserted_by_attempt_id: attempt_id.clone(),
            daily_retry_history: HashMap::from([(reference_time, 1)]),
            scheduled_at: None,
            is_hard_decline: Some(is_hard_decline),
            modified_at: Some(recovery_attempt.created_at),
            payment_processor_token_details: PaymentProcessorTokenDetails {
                payment_processor_token: revenue_recovery_attempt_data
                    .processor_payment_method_token
                    .clone(),
                expiry_month: revenue_recovery_attempt_data
                    .card_info
                    .card_exp_month
                    .clone(),
                expiry_year: revenue_recovery_attempt_data
                    .card_info
                    .card_exp_year
                    .clone(),
                card_issuer: revenue_recovery_attempt_data.card_info.card_issuer.clone(),
                last_four_digits: revenue_recovery_attempt_data.card_info.last4.clone(),
                card_network: revenue_recovery_attempt_data.card_info.card_network.clone(),
                card_type: revenue_recovery_attempt_data.card_info.card_type.clone(),
                card_isin: revenue_recovery_attempt_data.card_info.card_isin.clone(),
            },
            is_active: Some(true), // Tokens created from recovery attempts are active by default
            account_update_history: None, // No prior account update history exists for freshly ingested tokens
            decision_threshold: None,
        };

        // Make the Redis call to store tokens
        RedisTokenManager::upsert_payment_processor_token(
            state,
            &connector_customer_id,
            token_unit,
        )
        .await
        .change_context(errors::RevenueRecoveryError::RevenueRecoveryRedisInsertFailed)
        .attach_printable("Failed to store payment processor tokens in Redis")?;

        Ok(())
    }
}

pub struct BillingConnectorPaymentsSyncResponseData(
    revenue_recovery_response::BillingConnectorPaymentsSyncResponse,
);
pub struct BillingConnectorPaymentsSyncFlowRouterData(
    router_types::BillingConnectorPaymentsSyncRouterData,
);

impl BillingConnectorPaymentsSyncResponseData {
    async fn handle_billing_connector_payment_sync_call(
        state: &SessionState,
        platform: &domain::Platform,
        merchant_connector_account: &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        connector_name: &str,
        id: &str,
    ) -> CustomResult<Self, errors::RevenueRecoveryError> {
        let connector_data = api::ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            connector_name,
            api::GetToken::Connector,
            None,
        )
        .change_context(errors::RevenueRecoveryError::BillingConnectorPaymentsSyncFailed)
        .attach_printable("invalid connector name received in payment attempt")?;

        let connector_integration: services::BoxedBillingConnectorPaymentsSyncIntegrationInterface<
            router_flow_types::BillingConnectorPaymentsSync,
            revenue_recovery_request::BillingConnectorPaymentsSyncRequest,
            revenue_recovery_response::BillingConnectorPaymentsSyncResponse,
        > = connector_data.connector.get_connector_integration();

        let router_data =
            BillingConnectorPaymentsSyncFlowRouterData::construct_router_data_for_billing_connector_payment_sync_call(
                state,
                connector_name,
                merchant_connector_account,
                platform,
                id,
            )
            .await
            .change_context(errors::RevenueRecoveryError::BillingConnectorPaymentsSyncFailed)
            .attach_printable(
                "Failed while constructing router data for billing connector psync call",
            )?
            .inner();

        let response = services::execute_connector_processing_step(
            state,
            connector_integration,
            &router_data,
            payments::CallConnectorAction::Trigger,
            None,
            None,
        )
        .await
        .change_context(errors::RevenueRecoveryError::BillingConnectorPaymentsSyncFailed)
        .attach_printable("Failed while fetching billing connector payment details")?;

        let additional_recovery_details = match response.response {
            Ok(response) => Ok(response),
            error @ Err(_) => {
                logger::error!(?error);
                Err(errors::RevenueRecoveryError::BillingConnectorPaymentsSyncFailed)
                    .attach_printable("Failed while fetching billing connector payment details")
            }
        }?;
        Ok(Self(additional_recovery_details))
    }

    async fn get_billing_connector_payment_details(
        should_billing_connector_payment_api_called: bool,
        state: &SessionState,
        platform: &domain::Platform,
        billing_connector_account: &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        connector_name: &str,
        object_ref_id: &webhooks::ObjectReferenceId,
    ) -> CustomResult<
        Option<revenue_recovery_response::BillingConnectorPaymentsSyncResponse>,
        errors::RevenueRecoveryError,
    > {
        let response_data = match should_billing_connector_payment_api_called {
            true => {
                let billing_connector_transaction_id = object_ref_id
                    .clone()
                    .get_connector_transaction_id_as_string()
                    .change_context(
                        errors::RevenueRecoveryError::BillingConnectorPaymentsSyncFailed,
                    )
                    .attach_printable("Billing connector Payments api call failed")?;
                let billing_connector_payment_details =
                    Self::handle_billing_connector_payment_sync_call(
                        state,
                        platform,
                        billing_connector_account,
                        connector_name,
                        &billing_connector_transaction_id,
                    )
                    .await?;
                Some(billing_connector_payment_details.inner())
            }
            false => None,
        };

        Ok(response_data)
    }

    fn inner(self) -> revenue_recovery_response::BillingConnectorPaymentsSyncResponse {
        self.0
    }
}

impl BillingConnectorPaymentsSyncFlowRouterData {
    async fn construct_router_data_for_billing_connector_payment_sync_call(
        state: &SessionState,
        connector_name: &str,
        merchant_connector_account: &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        platform: &domain::Platform,
        billing_connector_psync_id: &str,
    ) -> CustomResult<Self, errors::RevenueRecoveryError> {
        let auth_type: types::ConnectorAuthType = helpers::MerchantConnectorAccountType::DbVal(
            Box::new(merchant_connector_account.clone()),
        )
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(errors::RevenueRecoveryError::BillingConnectorPaymentsSyncFailed)?;

        let connector = common_enums::connector_enums::Connector::from_str(connector_name)
            .change_context(errors::RevenueRecoveryError::BillingConnectorInvoiceSyncFailed)
            .attach_printable("Cannot find connector from the connector_name")?;

        let connector_params =
            hyperswitch_domain_models::connector_endpoints::Connectors::get_connector_params(
                &state.conf.connectors,
                connector,
            )
            .change_context(errors::RevenueRecoveryError::BillingConnectorPaymentsSyncFailed)
            .attach_printable(format!(
                "cannot find connector params for this connector {connector} in this flow",
            ))?;

        let router_data = types::RouterDataV2 {
            flow: PhantomData::<router_flow_types::BillingConnectorPaymentsSync>,
            tenant_id: state.tenant.tenant_id.clone(),
            resource_common_data: flow_common_types::BillingConnectorPaymentsSyncFlowData,
            connector_auth_type: auth_type,
            request: revenue_recovery_request::BillingConnectorPaymentsSyncRequest {
                connector_params,
                billing_connector_psync_id: billing_connector_psync_id.to_string(),
            },
            response: Err(types::ErrorResponse::default()),
        };

        let old_router_data =
            flow_common_types::BillingConnectorPaymentsSyncFlowData::to_old_router_data(
                router_data,
            )
            .change_context(errors::RevenueRecoveryError::BillingConnectorPaymentsSyncFailed)
            .attach_printable(
                "Cannot construct router data for making the billing connector payments api call",
            )?;

        Ok(Self(old_router_data))
    }

    fn inner(self) -> router_types::BillingConnectorPaymentsSyncRouterData {
        self.0
    }
}

pub struct BillingConnectorInvoiceSyncResponseData(
    revenue_recovery_response::BillingConnectorInvoiceSyncResponse,
);
pub struct BillingConnectorInvoiceSyncFlowRouterData(
    router_types::BillingConnectorInvoiceSyncRouterData,
);

impl BillingConnectorInvoiceSyncResponseData {
    async fn handle_billing_connector_invoice_sync_call(
        state: &SessionState,
        platform: &domain::Platform,
        merchant_connector_account: &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        connector_name: &str,
        id: &str,
    ) -> CustomResult<Self, errors::RevenueRecoveryError> {
        let connector_data = api::ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            connector_name,
            api::GetToken::Connector,
            None,
        )
        .change_context(errors::RevenueRecoveryError::BillingConnectorInvoiceSyncFailed)
        .attach_printable("invalid connector name received in payment attempt")?;

        let connector_integration: services::BoxedBillingConnectorInvoiceSyncIntegrationInterface<
            router_flow_types::BillingConnectorInvoiceSync,
            revenue_recovery_request::BillingConnectorInvoiceSyncRequest,
            revenue_recovery_response::BillingConnectorInvoiceSyncResponse,
        > = connector_data.connector.get_connector_integration();

        let router_data =
            BillingConnectorInvoiceSyncFlowRouterData::construct_router_data_for_billing_connector_invoice_sync_call(
                state,
                connector_name,
                merchant_connector_account,
                platform,
                id,
            )
            .await
            .change_context(errors::RevenueRecoveryError::BillingConnectorInvoiceSyncFailed)
            .attach_printable(
                "Failed while constructing router data for billing connector psync call",
            )?
            .inner();

        let response = services::execute_connector_processing_step(
            state,
            connector_integration,
            &router_data,
            payments::CallConnectorAction::Trigger,
            None,
            None,
        )
        .await
        .change_context(errors::RevenueRecoveryError::BillingConnectorInvoiceSyncFailed)
        .attach_printable("Failed while fetching billing connector Invoice details")?;

        let additional_recovery_details = match response.response {
            Ok(response) => Ok(response),
            error @ Err(_) => {
                logger::error!(?error);
                Err(errors::RevenueRecoveryError::BillingConnectorPaymentsSyncFailed)
                    .attach_printable("Failed while fetching billing connector Invoice details")
            }
        }?;
        Ok(Self(additional_recovery_details))
    }

    async fn get_billing_connector_invoice_details(
        should_billing_connector_invoice_api_called: bool,
        state: &SessionState,
        platform: &domain::Platform,
        billing_connector_account: &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        connector_name: &str,
        merchant_reference_id: Option<id_type::PaymentReferenceId>,
    ) -> CustomResult<
        Option<revenue_recovery_response::BillingConnectorInvoiceSyncResponse>,
        errors::RevenueRecoveryError,
    > {
        let response_data = match should_billing_connector_invoice_api_called {
            true => {
                let billing_connector_invoice_id = merchant_reference_id
                    .as_ref()
                    .map(|id| id.get_string_repr())
                    .ok_or(errors::RevenueRecoveryError::BillingConnectorInvoiceSyncFailed)?;

                let billing_connector_invoice_details =
                    Self::handle_billing_connector_invoice_sync_call(
                        state,
                        platform,
                        billing_connector_account,
                        connector_name,
                        billing_connector_invoice_id,
                    )
                    .await?;
                Some(billing_connector_invoice_details.inner())
            }
            false => None,
        };

        Ok(response_data)
    }

    fn inner(self) -> revenue_recovery_response::BillingConnectorInvoiceSyncResponse {
        self.0
    }
}

impl BillingConnectorInvoiceSyncFlowRouterData {
    async fn construct_router_data_for_billing_connector_invoice_sync_call(
        state: &SessionState,
        connector_name: &str,
        merchant_connector_account: &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        platform: &domain::Platform,
        billing_connector_invoice_id: &str,
    ) -> CustomResult<Self, errors::RevenueRecoveryError> {
        let auth_type: types::ConnectorAuthType = helpers::MerchantConnectorAccountType::DbVal(
            Box::new(merchant_connector_account.clone()),
        )
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(errors::RevenueRecoveryError::BillingConnectorInvoiceSyncFailed)?;

        let connector = common_enums::connector_enums::Connector::from_str(connector_name)
            .change_context(errors::RevenueRecoveryError::BillingConnectorInvoiceSyncFailed)
            .attach_printable("Cannot find connector from the connector_name")?;

        let connector_params =
            hyperswitch_domain_models::connector_endpoints::Connectors::get_connector_params(
                &state.conf.connectors,
                connector,
            )
            .change_context(errors::RevenueRecoveryError::BillingConnectorPaymentsSyncFailed)
            .attach_printable(format!(
                "cannot find connector params for this connector {connector} in this flow",
            ))?;

        let router_data = types::RouterDataV2 {
            flow: PhantomData::<router_flow_types::BillingConnectorInvoiceSync>,
            tenant_id: state.tenant.tenant_id.clone(),
            resource_common_data: flow_common_types::BillingConnectorInvoiceSyncFlowData,
            connector_auth_type: auth_type,
            request: revenue_recovery_request::BillingConnectorInvoiceSyncRequest {
                billing_connector_invoice_id: billing_connector_invoice_id.to_string(),
                connector_params,
            },
            response: Err(types::ErrorResponse::default()),
        };

        let old_router_data =
            flow_common_types::BillingConnectorInvoiceSyncFlowData::to_old_router_data(
                router_data,
            )
            .change_context(errors::RevenueRecoveryError::BillingConnectorInvoiceSyncFailed)
            .attach_printable(
                "Cannot construct router data for making the billing connector invoice api call",
            )?;

        Ok(Self(old_router_data))
    }

    fn inner(self) -> router_types::BillingConnectorInvoiceSyncRouterData {
        self.0
    }
}

#[derive(Clone, Debug)]
pub struct RecoveryPaymentTuple(
    revenue_recovery::RecoveryPaymentIntent,
    revenue_recovery::RecoveryPaymentAttempt,
);

impl RecoveryPaymentTuple {
    pub fn new(
        payment_intent: &revenue_recovery::RecoveryPaymentIntent,
        payment_attempt: &revenue_recovery::RecoveryPaymentAttempt,
    ) -> Self {
        Self(payment_intent.clone(), payment_attempt.clone())
    }

    pub async fn publish_revenue_recovery_event_to_kafka(
        state: &SessionState,
        recovery_payment_tuple: &Self,
        retry_count: Option<i32>,
    ) -> CustomResult<(), errors::RevenueRecoveryError> {
        let recovery_payment_intent = &recovery_payment_tuple.0;
        let recovery_payment_attempt = &recovery_payment_tuple.1;
        let revenue_recovery_feature_metadata = recovery_payment_intent
            .feature_metadata
            .as_ref()
            .and_then(|metadata| metadata.revenue_recovery.as_ref());

        let billing_city = recovery_payment_intent
            .billing_address
            .as_ref()
            .and_then(|billing_address| billing_address.address.as_ref())
            .and_then(|address| address.city.clone())
            .map(Secret::new);

        let billing_state = recovery_payment_intent
            .billing_address
            .as_ref()
            .and_then(|billing_address| billing_address.address.as_ref())
            .and_then(|address| address.state.clone());

        let billing_country = recovery_payment_intent
            .billing_address
            .as_ref()
            .and_then(|billing_address| billing_address.address.as_ref())
            .and_then(|address| address.country);

        let card_info = revenue_recovery_feature_metadata.and_then(|metadata| {
            metadata
                .billing_connector_payment_method_details
                .as_ref()
                .and_then(|details| details.get_billing_connector_card_info())
        });

        #[allow(clippy::as_conversions)]
        let retry_count = Some(retry_count.unwrap_or_else(|| {
            revenue_recovery_feature_metadata
                .map(|data| data.total_retry_count as i32)
                .unwrap_or(0)
        }));

        let event = kafka::revenue_recovery::RevenueRecovery {
            merchant_id: &recovery_payment_intent.merchant_id,
            invoice_amount: recovery_payment_intent.invoice_amount,
            invoice_currency: &recovery_payment_intent.invoice_currency,
            invoice_date: revenue_recovery_feature_metadata.and_then(|data| {
                data.invoice_billing_started_at_time
                    .map(|time| time.assume_utc())
            }),
            invoice_due_date: revenue_recovery_feature_metadata
                .and_then(|data| data.invoice_next_billing_time.map(|time| time.assume_utc())),
            billing_city,
            billing_country: billing_country.as_ref(),
            billing_state,
            attempt_amount: recovery_payment_attempt.amount,
            attempt_currency: &recovery_payment_intent.invoice_currency.clone(),
            attempt_status: &recovery_payment_attempt.attempt_status.clone(),
            pg_error_code: recovery_payment_attempt.error_code.clone(),
            network_advice_code: recovery_payment_attempt.network_advice_code.clone(),
            network_error_code: recovery_payment_attempt.network_decline_code.clone(),
            first_pg_error_code: revenue_recovery_feature_metadata
                .and_then(|data| data.first_payment_attempt_pg_error_code.clone()),
            first_network_advice_code: revenue_recovery_feature_metadata
                .and_then(|data| data.first_payment_attempt_network_advice_code.clone()),
            first_network_error_code: revenue_recovery_feature_metadata
                .and_then(|data| data.first_payment_attempt_network_decline_code.clone()),
            attempt_created_at: recovery_payment_attempt.created_at.assume_utc(),
            payment_method_type: revenue_recovery_feature_metadata
                .map(|data| &data.payment_method_type),
            payment_method_subtype: revenue_recovery_feature_metadata
                .map(|data| &data.payment_method_subtype),
            card_network: card_info
                .as_ref()
                .and_then(|info| info.card_network.as_ref()),
            card_issuer: card_info.and_then(|data| data.card_issuer.clone()),
            retry_count,
            payment_gateway: revenue_recovery_feature_metadata.map(|data| data.connector),
        };
        state.event_handler.log_event(&event);
        Ok(())
    }
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug)]
pub struct RecoveryAction {
    pub action: common_types::payments::RecoveryAction,
}

impl RecoveryAction {
    pub fn get_action(
        event_type: webhooks::IncomingWebhookEvent,
        attempt_triggered_by: Option<common_enums::TriggeredBy>,
    ) -> common_types::payments::RecoveryAction {
        match event_type {
            webhooks::IncomingWebhookEvent::PaymentIntentFailure
            | webhooks::IncomingWebhookEvent::PaymentIntentSuccess
            | webhooks::IncomingWebhookEvent::PaymentIntentProcessing
            | webhooks::IncomingWebhookEvent::PaymentIntentPartiallyFunded
            | webhooks::IncomingWebhookEvent::PaymentIntentCancelled
            | webhooks::IncomingWebhookEvent::PaymentIntentCancelFailure
            | webhooks::IncomingWebhookEvent::PaymentIntentAuthorizationSuccess
            | webhooks::IncomingWebhookEvent::PaymentIntentAuthorizationFailure
            | webhooks::IncomingWebhookEvent::PaymentIntentCaptureSuccess
            | webhooks::IncomingWebhookEvent::PaymentIntentCaptureFailure
            | webhooks::IncomingWebhookEvent::PaymentIntentExpired
            | webhooks::IncomingWebhookEvent::PaymentActionRequired
            | webhooks::IncomingWebhookEvent::EventNotSupported
            | webhooks::IncomingWebhookEvent::SourceChargeable
            | webhooks::IncomingWebhookEvent::SourceTransactionCreated
            | webhooks::IncomingWebhookEvent::RefundFailure
            | webhooks::IncomingWebhookEvent::RefundSuccess
            | webhooks::IncomingWebhookEvent::DisputeOpened
            | webhooks::IncomingWebhookEvent::DisputeExpired
            | webhooks::IncomingWebhookEvent::DisputeAccepted
            | webhooks::IncomingWebhookEvent::DisputeCancelled
            | webhooks::IncomingWebhookEvent::DisputeChallenged
            | webhooks::IncomingWebhookEvent::DisputeWon
            | webhooks::IncomingWebhookEvent::DisputeLost
            | webhooks::IncomingWebhookEvent::MandateActive
            | webhooks::IncomingWebhookEvent::MandateRevoked
            | webhooks::IncomingWebhookEvent::EndpointVerification
            | webhooks::IncomingWebhookEvent::PaymentIntentExtendAuthorizationSuccess
            | webhooks::IncomingWebhookEvent::PaymentIntentExtendAuthorizationFailure
            | webhooks::IncomingWebhookEvent::ExternalAuthenticationARes
            | webhooks::IncomingWebhookEvent::FrmApproved
            | webhooks::IncomingWebhookEvent::FrmRejected
            | webhooks::IncomingWebhookEvent::PayoutSuccess
            | webhooks::IncomingWebhookEvent::PayoutFailure
            | webhooks::IncomingWebhookEvent::PayoutProcessing
            | webhooks::IncomingWebhookEvent::PayoutCancelled
            | webhooks::IncomingWebhookEvent::PayoutCreated
            | webhooks::IncomingWebhookEvent::PayoutExpired
            | webhooks::IncomingWebhookEvent::PayoutReversed
            | webhooks::IncomingWebhookEvent::InvoiceGenerated
            | webhooks::IncomingWebhookEvent::SetupWebhook => {
                common_types::payments::RecoveryAction::InvalidAction
            }
            webhooks::IncomingWebhookEvent::RecoveryPaymentFailure => match attempt_triggered_by {
                Some(common_enums::TriggeredBy::Internal) => {
                    common_types::payments::RecoveryAction::NoAction
                }
                Some(common_enums::TriggeredBy::External) | None => {
                    common_types::payments::RecoveryAction::ScheduleFailedPayment
                }
            },
            webhooks::IncomingWebhookEvent::RecoveryPaymentSuccess => match attempt_triggered_by {
                Some(common_enums::TriggeredBy::Internal) => {
                    common_types::payments::RecoveryAction::NoAction
                }
                Some(common_enums::TriggeredBy::External) | None => {
                    common_types::payments::RecoveryAction::SuccessPaymentExternal
                }
            },
            webhooks::IncomingWebhookEvent::RecoveryPaymentPending => {
                common_types::payments::RecoveryAction::PendingPayment
            }
            webhooks::IncomingWebhookEvent::RecoveryInvoiceCancel => {
                common_types::payments::RecoveryAction::CancelInvoice
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn handle_action(
        &self,
        state: &SessionState,
        business_profile: &domain::Profile,
        platform: &domain::Platform,
        billing_connector_account: &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        mca_retry_threshold: u16,
        intent_retry_count: u16,
        recovery_tuple: &(
            Option<revenue_recovery::RecoveryPaymentAttempt>,
            revenue_recovery::RecoveryPaymentIntent,
        ),
    ) -> CustomResult<webhooks::WebhookResponseTracker, errors::RevenueRecoveryError> {
        match self.action {
            common_types::payments::RecoveryAction::CancelInvoice => todo!(),
            common_types::payments::RecoveryAction::ScheduleFailedPayment => {
                let recovery_algorithm_type = business_profile
                    .revenue_recovery_retry_algorithm_type
                    .ok_or(report!(
                        errors::RevenueRecoveryError::RetryAlgorithmTypeNotFound
                    ))?;
                match recovery_algorithm_type {
                    api_enums::RevenueRecoveryAlgorithmType::Monitoring => {
                        handle_monitoring_threshold(
                            state,
                            business_profile,
                            platform.get_processor().get_key_store(),
                        )
                        .await
                    }
                    revenue_recovery_retry_type => {
                        handle_schedule_failed_payment(
                            billing_connector_account,
                            intent_retry_count,
                            mca_retry_threshold,
                            state,
                            platform,
                            recovery_tuple,
                            business_profile,
                            revenue_recovery_retry_type,
                        )
                        .await
                    }
                }
            }
            common_types::payments::RecoveryAction::SuccessPaymentExternal => {
                logger::info!("Payment has been succeeded via external system");
                Ok(webhooks::WebhookResponseTracker::NoEffect)
            }
            common_types::payments::RecoveryAction::PendingPayment => {
                logger::info!(
                    "Pending transactions are not consumed by the revenue recovery webhooks"
                );
                Ok(webhooks::WebhookResponseTracker::NoEffect)
            }
            common_types::payments::RecoveryAction::NoAction => {
                logger::info!(
                    "No Recovery action is taken place for recovery event and attempt triggered_by"
                );
                Ok(webhooks::WebhookResponseTracker::NoEffect)
            }
            common_types::payments::RecoveryAction::InvalidAction => {
                logger::error!("Invalid Revenue recovery action state has been received");
                Ok(webhooks::WebhookResponseTracker::NoEffect)
            }
        }
    }
}
