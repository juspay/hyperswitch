use std::collections::BTreeMap;
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
use hyperswitch_masking::{ExposeInterface, PeekInterface, Secret};
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
use time::{Date, OffsetDateTime, Time};

#[cfg(feature = "v2")]
use crate::core::payments::operations;
use crate::core::revenue_recovery::retry_stats::document::{SlotCounter, StatsDocument};
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
    consts,
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
            None,
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
    superposition_client: &external_services::superposition::SuperpositionClient,
    dimensions: &crate::core::configs::dimension_state::DimensionsWithProcessorMerchantIdAndConnector,
    retry_count: i32,
) -> Option<time::PrimitiveDateTime> {
    let mapping = dimensions
        .get_pt_mapping_pcr_retries(db, superposition_client, None)
        .await;

    let time_delta = scheduler_utils::get_pcr_payments_retry_schedule_time(mapping, retry_count);

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
    billing_connector: common_enums::connector_enums::Connector,
    retry_algorithm_type: RevenueRecoveryAlgorithmType,
    retry_count: i32,
) -> CustomResult<PaymentProcessorTokenResponse, errors::ProcessTrackerError> {
    let mut payment_processor_token_response = PaymentProcessorTokenResponse::None;
    match retry_algorithm_type {
        RevenueRecoveryAlgorithmType::Monitoring => {
            logger::error!("Monitoring type found for Revenue Recovery retry payment");
        }

        RevenueRecoveryAlgorithmType::Cascading => {
            let dimensions = crate::core::configs::dimension_state::Dimensions::new()
                .with_processor_merchant_id(payment_intent.merchant_id.clone().into())
                .with_connector(billing_connector);
            let time = get_schedule_time_to_retry_mit_payments(
                state.store.as_ref(),
                state.superposition_service.as_ref(),
                &dimensions,
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

    match (locked_acquired, payment_intent.status) {
        (true, _) | (false, common_enums::IntentStatus::PartiallyCaptured) => {
            let payment_processor_token_response = get_payment_processor_token_by_calling_decider(
                state,
                payment_intent,
                connector_customer_id,
            )
            .await?;
            Ok(payment_processor_token_response)
        }
        (false, _) => {
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
    }
}

#[cfg(feature = "v2")]
async fn get_payment_processor_token_by_calling_decider(
    state: &SessionState,
    payment_intent: &PaymentIntent,
    connector_customer_id: &str,
) -> CustomResult<PaymentProcessorTokenResponse, errors::ProcessTrackerError> {
    // Get existing tokens from Redis
    let existing_tokens = RedisTokenManager::get_connector_customer_payment_processor_tokens(
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
            // Unlock the customer status only if all tokens are hard declined and payment intent is in Failed status
            let _unlocked = match payment_intent.status {
                common_enums::enums::IntentStatus::Failed => {
                    RedisTokenManager::unlock_connector_customer_status(
                        state,
                        connector_customer_id,
                        &payment_intent.id,
                    )
                    .await
                    .change_context(
                        errors::ProcessTrackerError::ERedisError(
                            errors::RedisError::RedisConnectionError.into(),
                        ),
                    )?
                }
                _ => false,
            };

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
        connector_name,
        REVENUE_RECOVERY,
        consts::DEFAULT_SUBFLOW_STR,
        error_code,
        error_message,
        None, // issuer_error_code not available in recovery context
        None, // card_network
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

// ---------------------------------------------------------------------------
// MathModel retry-time prediction — the data-driven half of the Cascading (MathModel) strategy.
//
// Given a cluster's day-of-week / day-of-month / hour-of-day success stats (`StatsDocument`), the
// remaining retry budget, and the grace window, it returns the datetime to retry on — via per-tick
// probabilistic firing (real randomness, no seed) with a runway guard. The caller `min()`s this
// with the Superposition static-schedule time (MathModel can only make a retry happen SOONER).
//
// ALWAYS returns a date (never None): Laplace smoothing gives every slot a defined estimate and the
// runway guard guarantees a pick even for sparse/empty stats.
//
// INDEXING (must match `retry_stats::document::EventSlots::from_utc`, which is how the stats are
// recorded):
//   * dow: `weekday().number_days_from_monday()`  →  MONDAY = 0 .. Sunday = 6
//   * dom: `day() - 1`                            →  0-indexed (0 = the 1st)
//   * hod: `hour()`                               →  0 .. 23 (UTC)
// ---------------------------------------------------------------------------

/// Clip C_ij to [CLIP, 1-CLIP] so near-certain comparisons don't send logit to +/- infinity.
const CLIP: f64 = 1e-4;
/// Fallback hour when the cluster has no hour-of-day data at all.
const DEFAULT_RETRY_HOUR: u8 = 12;
/// Max candidate-window length. Beyond ~a month the window only repeats weekday/month-day slots that
/// are already represented (all 7 weekdays — and all month-days except when the window spans February),
/// so a longer grace adds little new signal; this caps the work and guards a config typo. NOTE: the cap
/// is a behavior change (a grace > 31 can never propose days 32+), so it's logged where it triggers.
const MAX_GRACE_DAYS: u32 = 31;

/// Laplace-smoothed success rate: p̂ = (k+1)/(n+2). Callers must pass a well-formed counter (k ≤ n);
/// `slot_scores` DROPS corrupt `k > n` slots before this runs, so p̂ ∈ (0,1) strictly and `se` never
/// takes the sqrt of a negative. (Do NOT clamp k→n here — that reads as "all succeeded" and would make
/// a corrupt slot the best in the cluster; dropping degrades it to "no data" instead.)
#[allow(clippy::as_conversions)]
fn p_hat(c: SlotCounter) -> f64 {
    (c.k as f64 + 1.0) / (c.n as f64 + 2.0)
}

/// Beta-posterior standard deviation: SE = sqrt(p̂(1-p̂)/(n+3)).
#[allow(clippy::as_conversions)]
fn se(c: SlotCounter) -> f64 {
    let p = p_hat(c);
    (p * (1.0 - p) / (c.n as f64 + 3.0)).sqrt()
}

/// Standard normal CDF via erf (Abramowitz & Stegun 7.1.26, max err ~1.5e-7).
fn normal_cdf(x: f64) -> f64 {
    0.5 * (1.0 + erf(x / std::f64::consts::SQRT_2))
}

fn erf(x: f64) -> f64 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + 0.327_591_1 * x);
    let y = 1.0
        - (((((1.061_405_429 * t - 1.453_152_027) * t) + 1.421_413_741) * t - 0.284_496_736) * t
            + 0.254_829_592)
            * t
            * (-x * x).exp();
    sign * y
}

fn logit(c: f64) -> f64 {
    let c = c.clamp(CLIP, 1.0 - CLIP);
    (c / (1.0 - c)).ln()
}

/// Per-slot confidence score = average over the OTHER present slots of
/// `logit( P(this slot's true rate > that slot's true rate) )`.
///
/// `StatsDocument` stores each family as a fixed-length array, so the slot index IS the array
/// index — out-of-domain keys are impossible by construction. Two exclusions:
///  * `n == 0` — never attempted: excluded exactly like an absent slot (NOT treated as a
///    0.5-prior, which would beat real low-rate slots).
///  * `k > n`  — corrupt counters: dropped so garbage can't pollute real slots' scores as a peer.
fn slot_scores(slots: &[SlotCounter]) -> BTreeMap<u8, f64> {
    // Corrupt counters (k > n) are untrustworthy — exclude them from SCORING. This does NOT skip the
    // retry: the corrupt slot's days stay in the candidate window and still get picked, just at the
    // neutral "no data" weight (exp(0)=1) instead of a fabricated score. So even an all-corrupt cluster
    // still retries — uniformly, no preference — rather than being steered by garbage. (Do NOT clamp
    // k→n: that reads as "all succeeded" and lets a corrupt slot dominate the real ones.)
    // Logged at debug (not warn): a stale bad counter would otherwise fire per slot, per invoice,
    // forever — alert fatigue, no new info. TODO: emit a corrupt-slot metric as the durable signal.
    let corrupt: Vec<usize> = slots
        .iter()
        .enumerate()
        .filter(|(_, c)| c.k > c.n)
        .map(|(i, _)| i)
        .collect();
    if !corrupt.is_empty() {
        logger::debug!(
            slots = ?corrupt,
            "cluster_stats: corrupt slot counters (k > n) excluded from scoring"
        );
    }
    let scored: Vec<(u8, f64, f64)> = slots
        .iter()
        .enumerate()
        .filter(|(_, c)| c.n > 0 && c.k <= c.n)
        .filter_map(|(i, c)| u8::try_from(i).ok().map(|i| (i, p_hat(*c), se(*c))))
        .collect();
    let mut out = BTreeMap::new();
    for &(i, pi, sei) in &scored {
        let mut sum = 0.0;
        let mut cnt = 0.0;
        for &(j, pj, sej) in &scored {
            if i == j {
                continue;
            }
            // denom > 0 always: Laplace smoothing keeps every scored slot's SE strictly positive.
            let denom = (sei * sei + sej * sej).sqrt();
            let c = normal_cdf((pi - pj) / denom);
            sum += logit(c);
            cnt += 1.0;
        }
        out.insert(i, if cnt > 0.0 { sum / cnt } else { 0.0 });
    }
    out
}

/// Why `pick_index` landed on the index it returned — so the caller can attribute the pick honestly
/// (a forced or exhausted pick must NOT be logged as if the weights drove it).
#[derive(Clone, Copy)]
enum PickDriver {
    Weights,     // the weights drove the choice (normal, model-driven case)
    RunwayGuard, // forced because budget >= days remaining
    Exhausted,   // nothing fired (budget 0) — fell through to the last index
}

/// Per-tick probabilistic pick over an ordered list of non-negative WEIGHTS, with the runway guard.
/// Fires index k with probability `min(budget · weight_k / remaining_weight, 1)` (remaining_weight
/// via a suffix-sum); `budget >= remaining` forces a fire. Weights need not be normalized — only
/// their ratios matter. Always returns an index, plus WHY (see `PickDriver`).
#[allow(clippy::as_conversions, clippy::indexing_slicing)]
fn pick_index(weights: &[f64], budget: u32) -> (usize, PickDriver) {
    let n = weights.len();
    if n == 0 {
        return (0, PickDriver::Exhausted);
    }
    let mut suffix = vec![0.0_f64; n + 1];
    for k in (0..n).rev() {
        suffix[k] = suffix[k + 1] + weights[k];
    }
    for k in 0..n {
        let remaining = n - k;
        let guard = (budget as usize) >= remaining; // runway guard forces the fire
        let p = if guard {
            1.0
        } else {
            (f64::from(budget) * weights[k] / suffix[k]).min(1.0)
        };
        if rand::random::<f64>() < p {
            return (
                k,
                if guard {
                    PickDriver::RunwayGuard
                } else {
                    PickDriver::Weights
                },
            );
        }
    }
    (n - 1, PickDriver::Exhausted)
}

/// Pick the hour the SAME way as days: per-tick over hours 0..23 with budget 1 (exactly one hour).
/// Missing hours score 0. Falls back to `DEFAULT_RETRY_HOUR` when there is no USABLE hour data —
/// guard on the SCORED slots, not the raw counters, so all-corrupt counters still fall back (an
/// all-zero array yields empty scores too, so this one check covers all no-data shapes).
#[allow(clippy::as_conversions)]
fn pick_hour(hod: &[SlotCounter]) -> u8 {
    let scores = slot_scores(hod);
    if scores.is_empty() {
        return DEFAULT_RETRY_HOUR;
    }
    let weights: Vec<f64> = (0u8..24)
        .map(|h| scores.get(&h).copied().unwrap_or(0.0).exp())
        .collect();
    pick_index(&weights, 1).0 as u8
}

/// Softmax `xs` (numerically stabilized by subtracting the max). `xs` is non-empty here.
fn softmax(xs: &[f64]) -> Vec<f64> {
    let m = xs.iter().copied().fold(f64::MIN, f64::max);
    let exps: Vec<f64> = xs.iter().map(|x| (x - m).exp()).collect();
    let z: f64 = exps.iter().sum();
    exps.iter().map(|e| e / z).collect()
}

/// THE COMBINE SEAM. Fold the day-of-week and day-of-month signals into one weight per candidate day.
///
/// v1: softmax each axis over the CANDIDATE DAYS, then take the max — "this day is good if either its
/// weekday OR its month-day is historically good." Simplest defensible combine; since the result is
/// `min()`d with the static schedule downstream, the downside is bounded. Known trade-offs accepted
/// for v1: `max` optimism (a day strong on one axis but weak on the other is picked on its strong
/// side) and a mild grace-dependent tilt toward day-of-month. To try a better combine later
/// (max-at-score / sum-of-logits / posterior sampling), change ONLY this function.
///
/// Returns per-day `(weight, winning_axis)`; the winner lets the caller log which signal drove a pick.
#[allow(clippy::indexing_slicing)]
fn combine_day_weight(
    dates: &[Date],
    dow: &BTreeMap<u8, f64>,
    dom: &BTreeMap<u8, f64>,
) -> (Vec<f64>, Vec<&'static str>) {
    let dow_sc: Vec<f64> = dates
        .iter()
        .map(|d| {
            dow.get(&d.weekday().number_days_from_monday())
                .copied()
                .unwrap_or(0.0)
        })
        .collect();
    let dom_sc: Vec<f64> = dates
        .iter()
        .map(|d| dom.get(&d.day().saturating_sub(1)).copied().unwrap_or(0.0))
        .collect();
    let p_dow = softmax(&dow_sc);
    let p_dom = softmax(&dom_sc);
    (0..dates.len())
        .map(|i| {
            let (pw, pm) = (p_dow[i], p_dom[i]);
            let winner = if pm > pw {
                "dom"
            } else if pw > pm {
                "dow"
            } else {
                "tie" // both axes equal (e.g. a cold cluster: both uniform) — neither "won"
            };
            (pw.max(pm), winner)
        })
        .unzip()
}

/// Predict the retry datetime from cluster stats.
///
/// * `stats`      the cluster JSON (dow/dom/hod `{n,k}` counters)
/// * `budget`     retries remaining (drives the runway guard)
/// * `grace_days` grace period, in days, COUNTING the failure day (today). Retriable window = the
///                `grace_days - 1` future days, capped at 31.
///
/// The window starts on the **NEXT day** (failure day + 1), never on the failure day itself — the
/// charge just failed today, so a same-day retry is low value. The min-gap / past-time guard rails
/// are applied by the CALLER on the returned time (per the MathModel design).
///
/// Returns `None` when `grace_days <= 1` (no future day inside the grace period); otherwise `Some`.
/// V1 LIMITATION: a grace of 1 (today only) with retries still available is treated as "no retry"; a
/// later version will handle that edge (e.g. a same-day retry after a delay). Uses real randomness
/// (no seed). The caller `min()`s the result with the static schedule time.
#[allow(clippy::as_conversions, clippy::indexing_slicing)]
#[instrument(skip_all)]
pub fn compute_mathmodel_retry_time(
    stats: &StatsDocument,
    budget: u32,
    grace_days: u32,
) -> Option<OffsetDateTime> {
    // `grace_days` COUNTS the failure day (today), which we never retry on — so the retriable window
    // is the `grace_days - 1` future days [failure_day + 1 .. failure_day + grace_days - 1]. When that
    // is empty (grace_days <= 1: no grace, or grace covers only today) there is no in-grace future day.
    // V1: return None here. A later version will handle "grace 1 but retries remain" (e.g. a same-day
    // retry after a delay) rather than skipping.
    let future_days = grace_days.saturating_sub(1);
    if future_days == 0 {
        return None;
    }
    let now = OffsetDateTime::now_utc();
    let dow_scores = slot_scores(&stats.dow);
    let dom_scores = slot_scores(&stats.dom);

    // Cap the window at MAX_GRACE_DAYS: beyond ~a month it only repeats already-covered weekday/
    // month-day slots. Truncation is a behavior change (days beyond the cap are never proposed); log
    // at debug, not warn — it fires per invoice for a mis-configured profile and warn would spam.
    if future_days > MAX_GRACE_DAYS {
        logger::debug!(
            configured = grace_days,
            capped = MAX_GRACE_DAYS,
            "mathmodel: grace window capped"
        );
    }
    let window_len = future_days.min(MAX_GRACE_DAYS) as usize;
    let start = now.date().next_day().unwrap_or(now.date());
    let mut dates = vec![start];
    while dates.len() < window_len {
        match dates[dates.len() - 1].next_day() {
            Some(next) => dates.push(next),
            None => break,
        }
    }

    let (day_weights, winners) = combine_day_weight(&dates, &dow_scores, &dom_scores);
    let (day_idx, pick_driver) = pick_index(&day_weights, budget);
    let hour = pick_hour(&stats.hod);
    let time = Time::from_hms(hour, 0, 0).unwrap_or(Time::MIDNIGHT);

    // Observability for a later recovery back-test: which signal drove the pick and how strong it was.
    // Guard-forced and budget-exhausted picks attribute to themselves (NOT the uninvolved softmax
    // winner), so the back-test never counts a non-model pick as a model pick.
    let driver = match pick_driver {
        PickDriver::Weights => winners[day_idx], // "dow" / "dom" / "tie"
        PickDriver::RunwayGuard => "runway_guard",
        PickDriver::Exhausted => "exhausted",
    };
    logger::debug!(
        chosen_day = %dates[day_idx],
        driver = driver,
        weight = day_weights[day_idx],
        "mathmodel: day pick"
    );

    Some(dates[day_idx].with_time(time).assume_offset(now.offset()))
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod mathmodel_retry_time_tests {
    use super::*;

    fn ctr(n: u64, k: u64) -> SlotCounter {
        SlotCounter { n, k }
    }

    fn doc_with(
        dow: &[(usize, u64, u64)],
        dom: &[(usize, u64, u64)],
        hod: &[(usize, u64, u64)],
    ) -> StatsDocument {
        let mut doc = StatsDocument::default();
        for &(i, n, k) in dow {
            doc.dow[i] = ctr(n, k);
        }
        for &(i, n, k) in dom {
            doc.dom[i] = ctr(n, k);
        }
        for &(i, n, k) in hod {
            doc.hod[i] = ctr(n, k);
        }
        doc
    }

    fn sample() -> StatsDocument {
        doc_with(
            &[(0, 1843, 512), (4, 1998, 671), (6, 987, 240)],
            &[(0, 2210, 1104), (14, 733, 156), (30, 1631, 799)],
            &[(9, 1120, 342), (10, 1345, 460), (22, 512, 108)],
        )
    }

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn scores_match_hand_math() {
        let doc = sample();
        let sc = slot_scores(&doc.dow);
        assert!(approx(sc[&4], 9.2103, 0.1), "score_4={}", sc[&4]);
        assert!(approx(sc[&0], -2.73, 0.1), "score_0={}", sc[&0]);
        assert!(approx(sc[&6], -6.48, 0.1), "score_6={}", sc[&6]);
    }

    #[test]
    fn combine_maps_scores_to_the_right_weekday() {
        // Exercises the indexing contract (dow index = number_days_from_monday), not just the
        // `time` crate: give Monday (0) a dominant score, no month-day signal, and assert the Monday
        // date in the window wins the weight. Deterministic — combine_day_weight uses no RNG.
        let start =
            Date::from_calendar_date(2026, time::Month::August, 17).expect("valid calendar date"); // a Monday
        let mut dates = vec![start];
        while dates.len() < 7 {
            dates.push(
                dates[dates.len() - 1]
                    .next_day()
                    .expect("next calendar day exists"),
            );
        }
        let dow = BTreeMap::from([(0u8, 5.0)]); // Monday dominant
        let dom = BTreeMap::<u8, f64>::new(); // no month-day signal
        let (weights, _) = combine_day_weight(&dates, &dow, &dom);
        let argmax = (0..dates.len())
            .max_by(|&a, &b| {
                weights[a]
                    .partial_cmp(&weights[b])
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(0);
        assert_eq!(
            dates[argmax], start,
            "Monday's dominant score must make the Monday date win"
        );
        assert_eq!(dates[argmax].weekday().number_days_from_monday(), 0);
    }

    #[test]
    fn pick_hour_is_valid_and_empty_defaults() {
        // No seed now, so assert only the invariant: a valid hour; no usable hour data -> default.
        let doc = sample();
        for _ in 0..100 {
            assert!(pick_hour(&doc.hod) < 24);
        }
        assert_eq!(pick_hour(&[SlotCounter::default(); 24]), DEFAULT_RETRY_HOUR);
    }

    #[test]
    fn result_is_always_in_window() {
        // Invariant for EVERY random outcome: result date ∈ [tomorrow, today+grace], hour ∈ 0..23,
        // never panics — for both rich and empty stats. (Window starts on the NEXT day; the bounds
        // carry a 1-day slack so a midnight tick between captures can't flake it.)
        let grace: u32 = 14;
        for stats in [sample(), StatsDocument::default()] {
            for _ in 0..200 {
                let before = OffsetDateTime::now_utc();
                let dt = compute_mathmodel_retry_time(&stats, 3, grace).expect("grace > 1 => Some");
                let last = (before + time::Duration::days(i64::from(grace) + 1)).date();
                assert!(
                    dt.date() > before.date() && dt.date() <= last,
                    "date {} out of window",
                    dt.date()
                );
                assert!(dt.hour() < 24);
            }
        }
    }

    #[test]
    fn window_starts_next_day() {
        // Failure day is excluded: the earliest candidate is tomorrow. grace COUNTS today, so grace 2
        // = today + 1 future day (tomorrow) — assert the pick is that next day, not the failure day.
        let before = OffsetDateTime::now_utc();
        let dt = compute_mathmodel_retry_time(&sample(), 3, 2).expect("grace 2 => Some");
        assert!(
            dt.date() > before.date(),
            "expected next day, got {} (today {})",
            dt.date(),
            before.date()
        );
        assert!(dt.date() <= (before + time::Duration::days(2)).date());
    }

    #[test]
    fn grace_zero_and_one_return_none() {
        // grace COUNTS today; grace 0 = no grace, grace 1 = today only -> no future day -> None (v1).
        assert!(compute_mathmodel_retry_time(&sample(), 3, 0).is_none());
        assert!(compute_mathmodel_retry_time(&sample(), 3, 1).is_none());
    }

    #[test]
    fn corrupt_counter_is_excluded_not_promoted() {
        // k > n is untrustworthy: the slot is DROPPED (not clamped to a perfect record), so it can't
        // dominate.
        let doc = doc_with(
            &[
                (0, 5, 5000),   // corrupt
                (4, 2000, 800), // real ~40%
                (6, 1800, 700), // real ~39%
            ],
            &[],
            &[],
        );
        let sc = slot_scores(&doc.dow);
        assert!(
            !sc.contains_key(&0),
            "corrupt slot must be excluded from scoring"
        );
        assert!(sc.contains_key(&4) && sc.contains_key(&6));
    }

    #[test]
    fn never_attempted_slots_are_excluded() {
        // n == 0 means "never attempted" (the array form of an absent slot): it must not
        // participate in scoring, otherwise its Laplace 0.5-prior would beat real low-rate slots.
        let doc = doc_with(&[(0, 1000, 100)], &[], &[]); // only Monday attempted, ~10%
        let sc = slot_scores(&doc.dow);
        assert_eq!(sc.len(), 1);
        assert!(!sc.contains_key(&1), "unattempted slot must be excluded");
    }

    #[test]
    fn all_corrupt_cluster_still_retries() {
        // Every slot corrupt (k > n) on all three axes -> all dropped -> uniform -> still a valid
        // in-window datetime, never a panic or a skipped retry.
        let corrupt = doc_with(&[(0, 1, 100), (3, 2, 50)], &[(5, 1, 80)], &[(9, 1, 30)]);
        let before = OffsetDateTime::now_utc();
        for _ in 0..50 {
            let dt = compute_mathmodel_retry_time(&corrupt, 3, 14).expect("grace > 1 => Some");
            assert!(dt.date() > before.date() && dt.hour() < 24);
        }
    }

    #[test]
    fn corrupt_hod_falls_back_to_default() {
        // All-corrupt hour counters -> no usable scores -> deterministic noon, not a
        // uniform-random hour.
        let doc = doc_with(&[], &[], &[(9, 1, 30)]);
        assert_eq!(pick_hour(&doc.hod), DEFAULT_RETRY_HOUR);
    }

    #[test]
    fn grace_is_capped_at_max() {
        let before = OffsetDateTime::now_utc();
        let dt = compute_mathmodel_retry_time(&sample(), 3, 365).expect("grace > 1 => Some");
        let last = (before + time::Duration::days(i64::from(MAX_GRACE_DAYS) + 1)).date();
        assert!(
            dt.date() <= last,
            "date {} exceeds capped window",
            dt.date()
        );
    }

    #[test]
    fn runway_guard_is_attributed() {
        // budget >= number of slots => the guard forces index 0 and reports itself (not "weights").
        let (idx, driver) = pick_index(&[1.0, 1.0, 1.0], 5);
        assert_eq!(idx, 0);
        assert!(matches!(driver, PickDriver::RunwayGuard));
    }
}
