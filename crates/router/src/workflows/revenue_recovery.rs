#[cfg(feature = "v2")]
use std::collections::HashMap;

#[cfg(feature = "v2")]
use api_models::{
    enums::{CardNetwork, RevenueRecoveryAlgorithmType},
    payments::PaymentsGetIntentRequest,
};
use common_utils::errors::CustomResult;
#[cfg(feature = "v2")]
use common_utils::{
    ext_traits::AsyncExt,
    ext_traits::{StringExt, ValueExt},
    id_type,
    pii::PhoneNumberStrategy,
};
#[cfg(feature = "v2")]
use diesel_models::types::BillingConnectorPaymentMethodDetails;
#[cfg(feature = "v2")]
use error_stack::{Report, ResultExt};
#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
use external_services::{
    date_time, grpc_client::revenue_recovery::recovery_decider_client as external_grpc_client,
};
#[cfg(feature = "v2")]
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    payments::{payment_attempt, PaymentConfirmData, PaymentIntent, PaymentIntentData},
    router_flow_types,
    router_flow_types::Authorize,
};
#[cfg(feature = "v2")]
use masking::{ExposeInterface, PeekInterface, Secret};
#[cfg(feature = "v2")]
use rand::Rng;
use router_env::{
    logger,
    tracing::{self, instrument},
};
use scheduler::{
    consumer::{self, workflows::ProcessTrackerWorkflow},
    errors,
};
#[cfg(feature = "v2")]
use scheduler::{types::process_data, utils as scheduler_utils};
#[cfg(feature = "v2")]
use storage_impl::errors as storage_errors;
#[cfg(feature = "v2")]
use time::Date;

#[cfg(feature = "v2")]
use crate::core::payments::operations;
#[cfg(feature = "v2")]
use crate::routes::app::ReqState;
#[cfg(feature = "v2")]
use crate::services;
#[cfg(feature = "v2")]
use crate::types::storage::{
    revenue_recovery::RetryLimitsConfig,
    revenue_recovery_redis_operation::{
        PaymentProcessorTokenStatus, PaymentProcessorTokenWithRetryInfo, RedisTokenManager,
    },
};
#[cfg(feature = "v2")]
use crate::workflows::revenue_recovery::pcr::api;
#[cfg(feature = "v2")]
use crate::{
    core::{
        payments,
        revenue_recovery::{self as pcr},
    },
    db::StorageInterface,
    errors::StorageError,
    types::{
        api::{self as api_types},
        domain,
        storage::{
            revenue_recovery as pcr_storage_types,
            revenue_recovery_redis_operation::PaymentProcessorTokenDetails,
        },
    },
};
use crate::{routes::SessionState, types::storage};
pub struct ExecutePcrWorkflow;
#[cfg(feature = "v2")]
pub const REVENUE_RECOVERY: &str = "revenue_recovery";
#[cfg(feature = "v2")]
const TOTAL_SLOTS_IN_MONTH: i32 = 720;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for ExecutePcrWorkflow {
    #[cfg(feature = "v1")]
    async fn execute_workflow<'a>(
        &'a self,
        _state: &'a SessionState,
        _process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        Ok(())
    }
    #[cfg(feature = "v2")]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let tracking_data = process
            .tracking_data
            .clone()
            .parse_value::<pcr_storage_types::RevenueRecoveryWorkflowTrackingData>(
            "PCRWorkflowTrackingData",
        )?;
        let request = PaymentsGetIntentRequest {
            id: tracking_data.global_payment_id.clone(),
        };
        let revenue_recovery_payment_data =
            extract_data_and_perform_action(state, &tracking_data).await?;
        let platform_from_revenue_recovery_payment_data = domain::Platform::new(
            revenue_recovery_payment_data.merchant_account.clone(),
            revenue_recovery_payment_data.key_store.clone(),
            revenue_recovery_payment_data.merchant_account.clone(),
            revenue_recovery_payment_data.key_store.clone(),
        );
        let (payment_data, _, _) = payments::payments_intent_operation_core::<
            api_types::PaymentGetIntent,
            _,
            _,
            PaymentIntentData<api_types::PaymentGetIntent>,
        >(
            state,
            state.get_req_state(),
            platform_from_revenue_recovery_payment_data.clone(),
            revenue_recovery_payment_data.profile.clone(),
            payments::operations::PaymentGetIntent,
            request,
            tracking_data.global_payment_id.clone(),
            hyperswitch_domain_models::payments::HeaderPayload::default(),
        )
        .await?;

        match process.name.as_deref() {
            Some("EXECUTE_WORKFLOW") => {
                Box::pin(pcr::perform_execute_payment(
                    state,
                    &process,
                    &revenue_recovery_payment_data.profile.clone(),
                    platform_from_revenue_recovery_payment_data.clone(),
                    &tracking_data,
                    &revenue_recovery_payment_data,
                    &payment_data.payment_intent,
                ))
                .await
            }
            Some("PSYNC_WORKFLOW") => {
                Box::pin(pcr::perform_payments_sync(
                    state,
                    &process,
                    &revenue_recovery_payment_data.profile.clone(),
                    platform_from_revenue_recovery_payment_data.clone(),
                    &tracking_data,
                    &revenue_recovery_payment_data,
                    &payment_data.payment_intent,
                ))
                .await?;
                Ok(())
            }
            Some("CALCULATE_WORKFLOW") => {
                Box::pin(pcr::perform_calculate_workflow(
                    state,
                    &process,
                    &revenue_recovery_payment_data.profile.clone(),
                    platform_from_revenue_recovery_payment_data,
                    &tracking_data,
                    &revenue_recovery_payment_data,
                    &payment_data.payment_intent,
                ))
                .await
            }

            _ => Err(errors::ProcessTrackerError::JobNotFound),
        }
    }
    #[instrument(skip_all)]
    async fn error_handler<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
        error: errors::ProcessTrackerError,
    ) -> CustomResult<(), errors::ProcessTrackerError> {
        logger::error!("Encountered error");
        consumer::consumer_error_handler(state.store.as_scheduler(), process, error).await
    }
}

#[cfg(feature = "v2")]
pub(crate) async fn extract_data_and_perform_action(
    state: &SessionState,
    tracking_data: &pcr_storage_types::RevenueRecoveryWorkflowTrackingData,
) -> Result<pcr_storage_types::RevenueRecoveryPaymentData, errors::ProcessTrackerError> {
    let db = &state.store;

    let key_store = db
        .get_merchant_key_store_by_merchant_id(
            &tracking_data.merchant_id,
            &db.get_master_key().to_vec().into(),
        )
        .await?;

    let merchant_account = db
        .find_merchant_account_by_merchant_id(&tracking_data.merchant_id, &key_store)
        .await?;

    let profile = db
        .find_business_profile_by_profile_id(&key_store, &tracking_data.profile_id)
        .await?;

    let billing_mca = db
        .find_merchant_connector_account_by_id(&tracking_data.billing_mca_id, &key_store)
        .await?;

    let pcr_payment_data = pcr_storage_types::RevenueRecoveryPaymentData {
        merchant_account,
        profile: profile.clone(),
        key_store,
        billing_mca,
        retry_algorithm: profile
            .revenue_recovery_retry_algorithm_type
            .unwrap_or(tracking_data.revenue_recovery_retry),
        psync_data: None,
    };
    Ok(pcr_payment_data)
}

#[cfg(feature = "v2")]
pub(crate) async fn get_schedule_time_to_retry_mit_payments(
    db: &dyn StorageInterface,
    merchant_id: &id_type::MerchantId,
    retry_count: i32,
) -> Option<time::PrimitiveDateTime> {
    let key = "pt_mapping_pcr_retries";
    let result = db
        .find_config_by_key(key)
        .await
        .map(|value| value.config)
        .and_then(|config| {
            config
                .parse_struct("RevenueRecoveryPaymentProcessTrackerMapping")
                .change_context(StorageError::DeserializationFailed)
        });

    let mapping = result.map_or_else(
        |error| {
            if error.current_context().is_db_not_found() {
                logger::debug!("Revenue Recovery retry config `{key}` not found, ignoring");
            } else {
                logger::error!(
                    ?error,
                    "Failed to read Revenue Recovery retry config `{key}`"
                );
            }
            process_data::RevenueRecoveryPaymentProcessTrackerMapping::default()
        },
        |mapping| {
            logger::debug!(?mapping, "Using custom pcr payments retry config");
            mapping
        },
    );

    let time_delta =
        scheduler_utils::get_pcr_payments_retry_schedule_time(mapping, merchant_id, retry_count);

    scheduler_utils::get_time_from_delta(time_delta)
}

#[derive(Debug, Clone)]
pub struct RetryDecision {
    pub retry_time: time::PrimitiveDateTime,
    pub decision_threshold: Option<f64>,
}

#[cfg(feature = "v2")]
pub(crate) async fn get_schedule_time_for_smart_retry(
    state: &SessionState,
    payment_intent: &PaymentIntent,
    retry_after_time: Option<prost_types::Timestamp>,
    token_with_retry_info: &PaymentProcessorTokenWithRetryInfo,
) -> Result<Option<RetryDecision>, errors::ProcessTrackerError> {
    let card_config = &state.conf.revenue_recovery.card_config;

    // Not populating it right now
    let first_error_message = "None".to_string();
    let retry_count_left = token_with_retry_info.monthly_retry_remaining;
    let pg_error_code = token_with_retry_info.token_status.error_code.clone();

    let card_info = token_with_retry_info
        .token_status
        .payment_processor_token_details
        .clone();

    let billing_state = payment_intent
        .billing_address
        .as_ref()
        .and_then(|addr_enc| addr_enc.get_inner().address.as_ref())
        .and_then(|details| details.state.as_ref())
        .cloned();

    let revenue_recovery_metadata = payment_intent
        .feature_metadata
        .as_ref()
        .and_then(|metadata| metadata.payment_revenue_recovery_metadata.as_ref());

    let card_network = card_info.card_network.clone();
    let total_retry_count_within_network = card_config.get_network_config(card_network.clone());

    let card_network_str = card_network.map(|network| network.to_string());

    let card_issuer_str = card_info
        .card_issuer
        .clone()
        .filter(|card_issuer| !card_issuer.is_empty());

    let card_funding_str = match card_info.card_type.as_deref() {
        Some("card") => None,
        Some(s) => Some(s.to_string()),
        None => None,
    };

    let start_time_primitive = payment_intent.created_at;
    let recovery_timestamp_config = &state.conf.revenue_recovery.recovery_timestamp;

    let modified_start_time_primitive = start_time_primitive.saturating_add(
        time::Duration::seconds(recovery_timestamp_config.initial_timestamp_in_seconds),
    );

    let start_time_proto = date_time::convert_to_prost_timestamp(modified_start_time_primitive);

    let merchant_id = Some(payment_intent.merchant_id.get_string_repr().to_string());
    let invoice_amount = Some(
        payment_intent
            .amount_details
            .order_amount
            .get_amount_as_i64(),
    );
    let invoice_currency = Some(payment_intent.amount_details.currency.to_string());

    let billing_country = payment_intent
        .billing_address
        .as_ref()
        .and_then(|addr_enc| addr_enc.get_inner().address.as_ref())
        .and_then(|details| details.country.as_ref())
        .map(|country| country.to_string());

    let billing_city = payment_intent
        .billing_address
        .as_ref()
        .and_then(|addr_enc| addr_enc.get_inner().address.as_ref())
        .and_then(|details| details.city.as_ref())
        .cloned();

    let first_pg_error_code = revenue_recovery_metadata
        .and_then(|metadata| metadata.first_payment_attempt_pg_error_code.clone());
    let first_network_advice_code = revenue_recovery_metadata
        .and_then(|metadata| metadata.first_payment_attempt_network_advice_code.clone());
    let first_network_error_code = revenue_recovery_metadata
        .and_then(|metadata| metadata.first_payment_attempt_network_decline_code.clone());

    let invoice_due_date = revenue_recovery_metadata
        .and_then(|metadata| metadata.invoice_next_billing_time)
        .map(date_time::convert_to_prost_timestamp);

    let decider_request = InternalDeciderRequest {
        first_error_message,
        billing_state,
        card_funding: card_funding_str,
        card_network: card_network_str,
        card_issuer: card_issuer_str,
        invoice_start_time: Some(start_time_proto),
        retry_count: Some(token_with_retry_info.total_30_day_retries.into()),
        merchant_id,
        invoice_amount,
        invoice_currency,
        invoice_due_date,
        billing_country,
        billing_city,
        attempt_currency: None,
        attempt_status: None,
        attempt_amount: None,
        pg_error_code,
        network_advice_code: None,
        network_error_code: None,
        first_pg_error_code,
        first_network_advice_code,
        first_network_error_code,
        attempt_response_time: None,
        payment_method_type: None,
        payment_gateway: None,
        retry_count_left: Some(retry_count_left.into()),
        total_retry_count_within_network: Some(
            total_retry_count_within_network
                .max_retry_count_for_thirty_day
                .into(),
        ),
        first_error_msg_time: None,
        wait_time: retry_after_time,
        payment_id: Some(payment_intent.get_id().get_string_repr().to_string()),
        hourly_retry_history: Some(
            token_with_retry_info
                .token_status
                .daily_retry_history
                .clone(),
        ),
        previous_threshold: token_with_retry_info.token_status.decision_threshold,
    };

    if let Some(mut client) = state.grpc_client.recovery_decider_client.clone() {
        match client
            .decide_on_retry(decider_request.into(), state.get_recovery_grpc_headers())
            .await
        {
            Ok(grpc_response) => Ok(grpc_response
                .retry_flag
                .then_some(())
                .and(grpc_response.retry_time)
                .and_then(|prost_ts| {
                    match date_time::convert_from_prost_timestamp(&prost_ts) {
                        Ok(pdt) => {
                            let response = RetryDecision {
                                retry_time: pdt,
                                decision_threshold: grpc_response.decision_threshold,
                            };
                            Some(response)
                        }
                        Err(e) => {
                            logger::error!(
                                "Failed to convert retry_time from prost::Timestamp: {e:?}"
                            );
                            None // If conversion fails, treat as no valid retry time
                        }
                    }
                })),

            Err(e) => {
                logger::error!("Recovery decider gRPC call failed: {e:?}");
                Ok(None)
            }
        }
    } else {
        logger::debug!("Recovery decider client is not configured");
        Ok(None)
    }
}

#[cfg(feature = "v2")]
async fn should_force_schedule_due_to_missed_slots(
    state: &SessionState,
    card_network: Option<CardNetwork>,
    token_with_retry_info: &PaymentProcessorTokenWithRetryInfo,
) -> CustomResult<bool, StorageError> {
    // Check monthly retry remaining first
    let has_monthly_retries = token_with_retry_info.monthly_retry_remaining >= 1;

    // If no monthly retries available, don't force schedule
    if !has_monthly_retries {
        return Ok(false);
    }

    Ok(RedisTokenManager::find_nearest_date_from_current(
        &token_with_retry_info.token_status.daily_retry_history,
    )
    // Filter: only consider entries with actual retries (retry_count > 0)
    .filter(|(_, retry_count)| *retry_count > 0)
    .map(|(most_recent_date, _retry_count)| {
        let threshold_hours = TOTAL_SLOTS_IN_MONTH
            / state
                .conf
                .revenue_recovery
                .card_config
                .get_network_config(card_network.clone())
                .max_retry_count_for_thirty_day;

        // Calculate time difference since last retry and compare with threshold
        (time::OffsetDateTime::now_utc() - most_recent_date.assume_utc()).whole_hours()
            > threshold_hours.into()
    })
    // Default to false if no valid retry history found (either none exists or all have retry_count = 0)
    .unwrap_or(false))
}

#[cfg(feature = "v2")]
pub fn convert_hourly_retry_history(
    input: Option<HashMap<time::PrimitiveDateTime, i32>>,
) -> HashMap<String, i32> {
    let fmt = time::macros::format_description!(
        "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]"
    );

    match input {
        Some(map) => map
            .into_iter()
            .map(|(dt, count)| (dt.format(&fmt).unwrap_or(dt.to_string()), count))
            .collect(),
        None => HashMap::new(),
    }
}

#[cfg(feature = "v2")]
#[derive(Debug)]
struct InternalDeciderRequest {
    first_error_message: String,
    billing_state: Option<Secret<String>>,
    card_funding: Option<String>,
    card_network: Option<String>,
    card_issuer: Option<String>,
    invoice_start_time: Option<prost_types::Timestamp>,
    retry_count: Option<i64>,
    merchant_id: Option<String>,
    invoice_amount: Option<i64>,
    invoice_currency: Option<String>,
    invoice_due_date: Option<prost_types::Timestamp>,
    billing_country: Option<String>,
    billing_city: Option<String>,
    attempt_currency: Option<String>,
    attempt_status: Option<String>,
    attempt_amount: Option<i64>,
    pg_error_code: Option<String>,
    network_advice_code: Option<String>,
    network_error_code: Option<String>,
    first_pg_error_code: Option<String>,
    first_network_advice_code: Option<String>,
    first_network_error_code: Option<String>,
    attempt_response_time: Option<prost_types::Timestamp>,
    payment_method_type: Option<String>,
    payment_gateway: Option<String>,
    retry_count_left: Option<i64>,
    total_retry_count_within_network: Option<i64>,
    first_error_msg_time: Option<prost_types::Timestamp>,
    wait_time: Option<prost_types::Timestamp>,
    payment_id: Option<String>,
    hourly_retry_history: Option<HashMap<time::PrimitiveDateTime, i32>>,
    previous_threshold: Option<f64>,
}

#[cfg(feature = "v2")]
impl From<InternalDeciderRequest> for external_grpc_client::DeciderRequest {
    fn from(internal_request: InternalDeciderRequest) -> Self {
        Self {
            first_error_message: internal_request.first_error_message,
            billing_state: internal_request.billing_state.map(|s| s.peek().to_string()),
            card_funding: internal_request.card_funding,
            card_network: internal_request.card_network,
            card_issuer: internal_request.card_issuer,
            invoice_start_time: internal_request.invoice_start_time,
            retry_count: internal_request.retry_count,
            merchant_id: internal_request.merchant_id,
            invoice_amount: internal_request.invoice_amount,
            invoice_currency: internal_request.invoice_currency,
            invoice_due_date: internal_request.invoice_due_date,
            billing_country: internal_request.billing_country,
            billing_city: internal_request.billing_city,
            attempt_currency: internal_request.attempt_currency,
            attempt_status: internal_request.attempt_status,
            attempt_amount: internal_request.attempt_amount,
            pg_error_code: internal_request.pg_error_code,
            network_advice_code: internal_request.network_advice_code,
            network_error_code: internal_request.network_error_code,
            first_pg_error_code: internal_request.first_pg_error_code,
            first_network_advice_code: internal_request.first_network_advice_code,
            first_network_error_code: internal_request.first_network_error_code,
            attempt_response_time: internal_request.attempt_response_time,
            payment_method_type: internal_request.payment_method_type,
            payment_gateway: internal_request.payment_gateway,
            retry_count_left: internal_request.retry_count_left,
            total_retry_count_within_network: internal_request.total_retry_count_within_network,
            first_error_msg_time: internal_request.first_error_msg_time,
            wait_time: internal_request.wait_time,
            payment_id: internal_request.payment_id,
            hourly_retry_history: convert_hourly_retry_history(
                internal_request.hourly_retry_history,
            ),
            previous_threshold: internal_request.previous_threshold,
        }
    }
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone)]
pub struct ScheduledToken {
    pub token_details: PaymentProcessorTokenDetails,
    pub retry_decision: RetryDecision,
}

#[cfg(feature = "v2")]
#[derive(Debug)]
struct TokenProcessResult {
    scheduled_token: Option<ScheduledToken>,
    force_scheduled: bool,
}

#[cfg(feature = "v2")]
pub fn calculate_difference_in_seconds(scheduled_time: time::PrimitiveDateTime) -> i64 {
    let now_utc = time::OffsetDateTime::now_utc();

    let scheduled_offset_dt = scheduled_time.assume_utc();
    let difference = scheduled_offset_dt - now_utc;

    difference.whole_seconds()
}

#[cfg(feature = "v2")]
pub async fn update_token_expiry_based_on_schedule_time(
    state: &SessionState,
    connector_customer_id: &str,
    delayed_schedule_time: time::PrimitiveDateTime,
) -> CustomResult<(), errors::ProcessTrackerError> {
    let expiry_buffer = state
        .conf
        .revenue_recovery
        .recovery_timestamp
        .redis_ttl_buffer_in_seconds;

    let expiry_time = calculate_difference_in_seconds(delayed_schedule_time) + expiry_buffer;
    RedisTokenManager::update_connector_customer_lock_ttl(
        state,
        connector_customer_id,
        expiry_time,
    )
    .await
    .change_context(errors::ProcessTrackerError::ERedisError(
        errors::RedisError::RedisConnectionError.into(),
    ));

    Ok(())
}

#[cfg(feature = "v2")]
#[derive(Debug)]
pub enum PaymentProcessorTokenResponse {
    /// Token HardDecline
    HardDecline,

    /// Token can be retried at this specific time
    ScheduledTime {
        scheduled_time: time::PrimitiveDateTime,
    },

    /// Token locked or unavailable, next attempt possible
    NextAvailableTime {
        next_available_time: time::PrimitiveDateTime,
    },

    /// No retry info available / nothing to do yet
    None,
}

#[cfg(feature = "v2")]
pub async fn get_token_with_schedule_time_based_on_retry_algorithm_type(
    state: &SessionState,
    connector_customer_id: &str,
    payment_intent: &PaymentIntent,
    retry_algorithm_type: RevenueRecoveryAlgorithmType,
    retry_count: i32,
) -> CustomResult<PaymentProcessorTokenResponse, errors::ProcessTrackerError> {
    let mut payment_processor_token_response = PaymentProcessorTokenResponse::None;
    match retry_algorithm_type {
        RevenueRecoveryAlgorithmType::Monitoring => {
            logger::error!("Monitoring type found for Revenue Recovery retry payment");
        }

        RevenueRecoveryAlgorithmType::Cascading => {
            let time = get_schedule_time_to_retry_mit_payments(
                state.store.as_ref(),
                &payment_intent.merchant_id,
                retry_count,
            )
            .await
            .ok_or(errors::ProcessTrackerError::EApiErrorResponse)?;

            let payment_processor_token = payment_intent
                .feature_metadata
                .as_ref()
                .and_then(|metadata| metadata.payment_revenue_recovery_metadata.as_ref())
                .map(|recovery_metadata| {
                    recovery_metadata
                        .billing_connector_payment_details
                        .payment_processor_token
                        .clone()
                });

            let payment_processor_tokens_details =
                RedisTokenManager::get_payment_processor_metadata_for_connector_customer(
                    state,
                    connector_customer_id,
                )
                .await
                .change_context(errors::ProcessTrackerError::ERedisError(
                    errors::RedisError::RedisConnectionError.into(),
                ))?;

            // Get the token info from redis
            let payment_processor_tokens_details_with_retry_info = payment_processor_token
                .as_ref()
                .and_then(|t| payment_processor_tokens_details.get(t));

            // If payment_processor_tokens_details_with_retry_info is None, then no schedule time
            match payment_processor_tokens_details_with_retry_info {
                None => {
                    payment_processor_token_response = PaymentProcessorTokenResponse::None;
                    logger::debug!("No payment processor token found for cascading retry");
                }
                Some(payment_token) => {
                    if payment_token.token_status.is_hard_decline.unwrap_or(false) {
                        payment_processor_token_response =
                            PaymentProcessorTokenResponse::HardDecline;
                    } else if payment_token.retry_wait_time_hours > 0 {
                        let utc_schedule_time: time::OffsetDateTime =
                            time::OffsetDateTime::now_utc()
                                + time::Duration::hours(payment_token.retry_wait_time_hours);
                        let next_available_time = time::PrimitiveDateTime::new(
                            utc_schedule_time.date(),
                            utc_schedule_time.time(),
                        );

                        payment_processor_token_response =
                            PaymentProcessorTokenResponse::NextAvailableTime {
                                next_available_time,
                            };
                    } else {
                        payment_processor_token_response =
                            PaymentProcessorTokenResponse::ScheduledTime {
                                scheduled_time: time,
                            };
                    }
                }
            }
        }

        RevenueRecoveryAlgorithmType::Smart => {
            payment_processor_token_response = get_best_psp_token_available_for_smart_retry(
                state,
                connector_customer_id,
                payment_intent,
            )
            .await
            .change_context(errors::ProcessTrackerError::EApiErrorResponse)?;
        }
    }

    match &mut payment_processor_token_response {
        PaymentProcessorTokenResponse::HardDecline => {
            logger::debug!("Token is hard declined");
        }

        PaymentProcessorTokenResponse::ScheduledTime { scheduled_time } => {
            // Add random delay to schedule time
            *scheduled_time = add_random_delay_to_schedule_time(state, *scheduled_time);

            // Log the scheduled retry time at debug level
            logger::info!("Retry scheduled at {:?}", scheduled_time);

            // Update token expiry based on schedule time
            update_token_expiry_based_on_schedule_time(
                state,
                connector_customer_id,
                *scheduled_time,
            )
            .await;
        }

        PaymentProcessorTokenResponse::NextAvailableTime {
            next_available_time,
        } => {
            logger::info!("Next available retry at {:?}", next_available_time);
        }

        PaymentProcessorTokenResponse::None => {
            logger::debug!("No retry info available");
        }
    }

    Ok(payment_processor_token_response)
}

#[cfg(feature = "v2")]
pub async fn get_best_psp_token_available_for_smart_retry(
    state: &SessionState,
    connector_customer_id: &str,
    payment_intent: &PaymentIntent,
) -> CustomResult<PaymentProcessorTokenResponse, errors::ProcessTrackerError> {
    //  Lock using payment_id
    let locked_acquired = RedisTokenManager::lock_connector_customer_status(
        state,
        connector_customer_id,
        &payment_intent.id,
    )
    .await
    .change_context(errors::ProcessTrackerError::ERedisError(
        errors::RedisError::RedisConnectionError.into(),
    ))?;

    match locked_acquired {
        false => {
            let token_details =
                RedisTokenManager::get_payment_processor_metadata_for_connector_customer(
                    state,
                    connector_customer_id,
                )
                .await
                .change_context(errors::ProcessTrackerError::ERedisError(
                    errors::RedisError::RedisConnectionError.into(),
                ))?;

            // Check token with schedule time in Redis
            let token_info_with_schedule_time = token_details
                .values()
                .find(|info| info.token_status.scheduled_at.is_some());

            // Check for hard decline if info is none
            let hard_decline_status = token_details
                .values()
                .all(|token| token.token_status.is_hard_decline.unwrap_or(false));

            let mut payment_processor_token_response = PaymentProcessorTokenResponse::None;

            if hard_decline_status {
                payment_processor_token_response = PaymentProcessorTokenResponse::HardDecline;
            } else {
                payment_processor_token_response = match token_info_with_schedule_time
                    .as_ref()
                    .and_then(|t| t.token_status.scheduled_at)
                {
                    Some(scheduled_time) => PaymentProcessorTokenResponse::NextAvailableTime {
                        next_available_time: scheduled_time,
                    },
                    None => PaymentProcessorTokenResponse::None,
                };
            }

            Ok(payment_processor_token_response)
        }

        true => {
            // Get existing tokens from Redis
            let existing_tokens =
                RedisTokenManager::get_connector_customer_payment_processor_tokens(
                    state,
                    connector_customer_id,
                )
                .await
                .change_context(errors::ProcessTrackerError::ERedisError(
                    errors::RedisError::RedisConnectionError.into(),
                ))?;

            let active_tokens: HashMap<_, _> = existing_tokens
                .into_iter()
                .filter(|(_, token_status)| token_status.is_active != Some(false))
                .collect();

            let result = RedisTokenManager::get_tokens_with_retry_metadata(state, &active_tokens);

            let payment_processor_token_response =
                call_decider_for_payment_processor_tokens_select_closest_time(
                    state,
                    &result,
                    payment_intent,
                    connector_customer_id,
                )
                .await
                .change_context(errors::ProcessTrackerError::EApiErrorResponse)?;

            Ok(payment_processor_token_response)
        }
    }
}

#[cfg(feature = "v2")]
pub async fn calculate_smart_retry_time(
    state: &SessionState,
    payment_intent: &PaymentIntent,
    token_with_retry_info: &PaymentProcessorTokenWithRetryInfo,
) -> Result<(Option<RetryDecision>, bool), errors::ProcessTrackerError> {
    let wait_hours = token_with_retry_info.retry_wait_time_hours;
    let current_time = time::OffsetDateTime::now_utc();
    let future_time = current_time + time::Duration::hours(wait_hours);

    // Timestamp after which retry can be done without penalty
    let future_timestamp = Some(prost_types::Timestamp {
        seconds: future_time.unix_timestamp(),
        nanos: 0,
    });

    let token = token_with_retry_info
        .token_status
        .payment_processor_token_details
        .payment_processor_token
        .clone();

    let masked_token: Secret<_, PhoneNumberStrategy> = Secret::new(token);

    let card_info = token_with_retry_info
        .token_status
        .payment_processor_token_details
        .clone();

    let card_network = card_info.card_network.clone();

    // Check if the last retry is not done within defined slot, force the retry to next slot
    if should_force_schedule_due_to_missed_slots(state, card_network.clone(), token_with_retry_info)
        .await
        .unwrap_or(false)
    {
        let schedule_offset = state
            .conf
            .revenue_recovery
            .recovery_timestamp
            .unretried_invoice_schedule_time_offset_seconds;
        let scheduled_time =
            time::OffsetDateTime::now_utc() + time::Duration::seconds(schedule_offset);
        logger::info!(
            "Skipping Decider call, forcing a schedule for the token:- '{:?}' to time:- {}",
            masked_token,
            scheduled_time
        );
        return Ok((
            Some(RetryDecision {
                retry_time: time::PrimitiveDateTime::new(
                    scheduled_time.date(),
                    scheduled_time.time(),
                ),
                // Not populating decision_threshold in forced schedule as there is no decider call
                decision_threshold: None,
            }),
            true, // force_scheduled
        ));
    }

    // Normal smart retry path
    let retry_decision = get_schedule_time_for_smart_retry(
        state,
        payment_intent,
        future_timestamp,
        token_with_retry_info,
    )
    .await?;

    Ok((retry_decision, false)) // force_scheduled = false
}

#[cfg(feature = "v2")]
async fn process_token_for_retry(
    state: &SessionState,
    token_with_retry_info: &PaymentProcessorTokenWithRetryInfo,
    payment_intent: &PaymentIntent,
) -> Result<TokenProcessResult, errors::ProcessTrackerError> {
    let token_status: &PaymentProcessorTokenStatus = &token_with_retry_info.token_status;
    let inserted_by_attempt_id = &token_status.inserted_by_attempt_id;

    let skip = token_status.is_hard_decline.unwrap_or(false);

    match skip {
        true => {
            logger::info!(
                "Skipping decider call due to hard decline token inserted by attempt_id: {}",
                inserted_by_attempt_id.get_string_repr()
            );
            Ok(TokenProcessResult {
                scheduled_token: None,
                force_scheduled: false,
            })
        }
        false => {
            let (retry_decision, force_scheduled) =
                calculate_smart_retry_time(state, payment_intent, token_with_retry_info).await?;

            Ok(TokenProcessResult {
                scheduled_token: retry_decision.map(|retry_decision| ScheduledToken {
                    token_details: token_status.payment_processor_token_details.clone(),
                    retry_decision,
                }),
                force_scheduled,
            })
        }
    }
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn call_decider_for_payment_processor_tokens_select_closest_time(
    state: &SessionState,
    processor_tokens: &HashMap<String, PaymentProcessorTokenWithRetryInfo>,
    payment_intent: &PaymentIntent,
    connector_customer_id: &str,
) -> CustomResult<PaymentProcessorTokenResponse, errors::ProcessTrackerError> {
    let mut tokens_with_schedule_time: Vec<ScheduledToken> = Vec::new();

    // Check for successful token
    let mut token_with_none_error_code = processor_tokens.values().find(|token| {
        token.token_status.error_code.is_none()
            && !token.token_status.is_hard_decline.unwrap_or(false)
    });

    match token_with_none_error_code {
        Some(token_with_retry_info) => {
            let token_details = &token_with_retry_info
                .token_status
                .payment_processor_token_details;

            let utc_schedule_time = time::OffsetDateTime::now_utc() + time::Duration::minutes(1);
            let schedule_time =
                time::PrimitiveDateTime::new(utc_schedule_time.date(), utc_schedule_time.time());

            tokens_with_schedule_time = vec![ScheduledToken {
                token_details: token_details.clone(),
                retry_decision: RetryDecision {
                    retry_time: schedule_time,
                    // Not populating decision_threshold for successful token as there is no decider call
                    decision_threshold: None,
                },
            }];

            tracing::debug!(
                "Found payment processor token with no error code, scheduling it for {schedule_time}",
            );
        }

        None => {
            // Flag to track if we found a force-scheduled token
            let mut force_scheduled_found = false;

            for token_with_retry_info in processor_tokens.values() {
                let result =
                    process_token_for_retry(state, token_with_retry_info, payment_intent).await?;

                // Add the scheduled token if it exists
                if let Some(scheduled_token) = result.scheduled_token {
                    tokens_with_schedule_time.push(scheduled_token);
                }

                // Check if this was force-scheduled due to missed slots
                if result.force_scheduled {
                    force_scheduled_found = true;
                    tracing::info!(
                        "Force-scheduled token detected due to missed slots, breaking early from token processing"
                    );
                    break; // Stop processing remaining tokens immediately
                }
            }
        }
    }

    let best_token = tokens_with_schedule_time
        .iter()
        .min_by_key(|token| token.retry_decision.retry_time)
        .cloned();

    let mut payment_processor_token_response;
    match best_token {
        None => {
            // No tokens available for scheduling, unlock the connector customer status

            // Check if all tokens are hard declined
            let hard_decline_status = processor_tokens
                .values()
                .all(|token| token.token_status.is_hard_decline.unwrap_or(false))
                && !processor_tokens.is_empty();

            RedisTokenManager::unlock_connector_customer_status(
                state,
                connector_customer_id,
                &payment_intent.id,
            )
            .await
            .change_context(errors::ProcessTrackerError::EApiErrorResponse)?;

            tracing::debug!("No payment processor tokens available for scheduling");

            if hard_decline_status {
                payment_processor_token_response = PaymentProcessorTokenResponse::HardDecline;
            } else {
                payment_processor_token_response = PaymentProcessorTokenResponse::None;
            }
        }

        Some(token) => {
            tracing::debug!("Found payment processor token with least schedule time");

            RedisTokenManager::update_payment_processor_tokens_schedule_time_to_none(
                state,
                connector_customer_id,
            )
            .await
            .change_context(errors::ProcessTrackerError::EApiErrorResponse)?;

            RedisTokenManager::update_payment_processor_token_schedule_time(
                state,
                connector_customer_id,
                &token.token_details.payment_processor_token,
                Some(token.retry_decision.retry_time),
                token.retry_decision.decision_threshold,
            )
            .await
            .change_context(errors::ProcessTrackerError::EApiErrorResponse)?;

            payment_processor_token_response = PaymentProcessorTokenResponse::ScheduledTime {
                scheduled_time: token.retry_decision.retry_time,
            };
        }
    }
    Ok(payment_processor_token_response)
}

#[cfg(feature = "v2")]
pub async fn check_hard_decline(
    state: &SessionState,
    payment_attempt: &payment_attempt::PaymentAttempt,
) -> Result<bool, error_stack::Report<storage_impl::errors::RecoveryError>> {
    let error_message = payment_attempt
        .error
        .as_ref()
        .map(|details| details.message.clone());

    let error_code = payment_attempt
        .error
        .as_ref()
        .map(|details| details.code.clone());

    let connector_name = payment_attempt
        .connector
        .clone()
        .ok_or(storage_impl::errors::RecoveryError::ValueNotFound)
        .attach_printable("unable to derive payment connector from payment attempt")?;

    let gsm_record = payments::helpers::get_gsm_record(
        state,
        error_code,
        error_message,
        connector_name,
        REVENUE_RECOVERY.to_string(),
    )
    .await;

    let is_hard_decline = gsm_record
        .and_then(|record| record.error_category)
        .map(|category| category == common_enums::ErrorCategory::HardDecline)
        .unwrap_or(false);

    Ok(is_hard_decline)
}

#[cfg(feature = "v2")]
pub fn add_random_delay_to_schedule_time(
    state: &SessionState,
    schedule_time: time::PrimitiveDateTime,
) -> time::PrimitiveDateTime {
    let mut rng = rand::thread_rng();
    let delay_limit = state
        .conf
        .revenue_recovery
        .recovery_timestamp
        .max_random_schedule_delay_in_seconds;
    let random_secs = rng.gen_range(1..=delay_limit);
    logger::info!("Adding random delay of {random_secs} seconds to schedule time");
    schedule_time + time::Duration::seconds(random_secs)
}
