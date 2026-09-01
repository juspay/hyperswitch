#[cfg(feature = "v2")]
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
use hyperswitch_domain_models::revenue_recovery::retry_stats_document::{
    SlotCounter, StatsDocument,
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
#[cfg(feature = "v2")]
use storage_impl::revenue_recovery_retry_stats::RevenueRecoveryRetryStatsInterface;

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

    /// The configured retry ladder has no slot left for this invoice
    RetriesExhausted,

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
    static_ladder_progress: &pcr::schedule::StaticLadderProgress,
) -> CustomResult<
    (
        PaymentProcessorTokenResponse,
        Option<pcr::schedule::StaticLadderProgress>,
    ),
    errors::ProcessTrackerError,
> {
    let mut payment_processor_token_response = PaymentProcessorTokenResponse::None;
    // Updated scheduling state, set only when a retry is actually scheduled by the adaptive
    // path. The other responses reschedule the CALCULATE job without making an attempt, so
    // persisting there would consume a pinned static time for a retry that never happened.
    let mut next_static_ladder_progress = None;
    match retry_algorithm_type {
        RevenueRecoveryAlgorithmType::Monitoring => {
            logger::error!("Monitoring type found for Revenue Recovery retry payment");
        }

        RevenueRecoveryAlgorithmType::Cascading => {
            let dimensions = crate::core::configs::dimension_state::Dimensions::new()
                .with_processor_merchant_id(payment_intent.merchant_id.clone().into())
                .with_connector(billing_connector);
            let schedule_time = get_schedule_time_to_retry_mit_payments(
                state.store.as_ref(),
                state.superposition_service.as_ref(),
                &dimensions,
                retry_count,
            )
            .await;

            // Distinct from `None` below, which means "no token right now, come back later" and
            // keeps the calculate job alive.
            let Some(time) = schedule_time else {
                logger::info!(retry_count, "Retry ladder exhausted for this invoice");
                return Ok((PaymentProcessorTokenResponse::RetriesExhausted, None));
            };

            payment_processor_token_response = get_token_availability_for_schedule_time(
                state,
                connector_customer_id,
                payment_intent,
                time,
            )
            .await?;
        }

        RevenueRecoveryAlgorithmType::Smart => {
            let dimensions = crate::core::configs::dimension_state::Dimensions::new()
                .with_processor_merchant_id(payment_intent.merchant_id.clone().into())
                .with_connector(billing_connector);

            let adaptive_retry_enabled = dimensions
                .get_adaptive_retry_enabled(
                    state.store.as_ref(),
                    state.superposition_service.as_ref(),
                    None,
                )
                .await;

            if adaptive_retry_enabled {
                // Same shape as the cascading arm — compute the schedule time, then gate on
                // the token. The only additions are the adaptive candidate and the choice
                // between the two.
                let queried_rung = static_ladder_progress.next_rung();
                let static_time = get_schedule_time_to_retry_mit_payments(
                    state.store.as_ref(),
                    state.superposition_service.as_ref(),
                    &dimensions,
                    queried_rung,
                )
                .await
                .ok_or(errors::ProcessTrackerError::EApiErrorResponse)?;

                // The one extra call. `None` whenever the algorithm has no opinion, in which
                // case the decision resolves to the static time exactly as cascading would.
                let adaptive_time: Option<time::PrimitiveDateTime> = None;

                let decision = pcr::schedule::decide_next_retry(
                    static_ladder_progress,
                    queried_rung,
                    static_time,
                    adaptive_time,
                );

                logger::info!(
                    source = ?decision.source,
                    queried_rung = queried_rung,
                    static_time = ?static_time,
                    schedule_time = ?decision.schedule_time,
                    "Adaptive retry decision"
                );

                payment_processor_token_response = get_token_availability_for_schedule_time(
                    state,
                    connector_customer_id,
                    payment_intent,
                    decision.schedule_time,
                )
                .await?;

                // The rung is consumed only when a retry is genuinely scheduled. The other
                // responses finish or reschedule the CALCULATE job without an attempt.
                if matches!(
                    payment_processor_token_response,
                    PaymentProcessorTokenResponse::ScheduledTime { .. }
                ) {
                    next_static_ladder_progress = Some(decision.next_progress);
                }
            } else {
                payment_processor_token_response = get_best_psp_token_available_for_smart_retry(
                    state,
                    connector_customer_id,
                    payment_intent,
                )
                .await
                .change_context(errors::ProcessTrackerError::EApiErrorResponse)?;
            }
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

        PaymentProcessorTokenResponse::RetriesExhausted => {
            logger::debug!("Retry ladder exhausted");
        }

        PaymentProcessorTokenResponse::None => {
            logger::debug!("No retry info available");
        }
    }

    Ok((
        payment_processor_token_response,
        next_static_ladder_progress,
    ))
}

#[cfg(feature = "v2")]
pub(crate) fn get_invoice_payment_processor_token(
    payment_intent: &PaymentIntent,
) -> Option<String> {
    payment_intent
        .feature_metadata
        .as_ref()
        .and_then(|metadata| metadata.payment_revenue_recovery_metadata.as_ref())
        .map(|recovery_metadata| {
            recovery_metadata
                .billing_connector_payment_details
                .payment_processor_token
                .clone()
        })
}

/// Check the invoice's payment processor token against a schedule time already decided.
/// Shared by the cascading and adaptive paths so both gate on the same conditions
#[cfg(feature = "v2")]
async fn get_token_availability_for_schedule_time(
    state: &SessionState,
    connector_customer_id: &str,
    payment_intent: &PaymentIntent,
    scheduled_time: time::PrimitiveDateTime,
) -> CustomResult<PaymentProcessorTokenResponse, errors::ProcessTrackerError> {
    let payment_processor_token = get_invoice_payment_processor_token(payment_intent);

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
        .and_then(|token| payment_processor_tokens_details.get(token));

    // If payment_processor_tokens_details_with_retry_info is None, then no schedule time
    let payment_processor_token_response = match payment_processor_tokens_details_with_retry_info {
        None => {
            logger::debug!("No payment processor token found for retry");
            PaymentProcessorTokenResponse::None
        }
        Some(payment_token) => {
            if payment_token.token_status.is_hard_decline.unwrap_or(false) {
                PaymentProcessorTokenResponse::HardDecline
            } else if payment_token.retry_wait_time_hours > 0 {
                let utc_schedule_time: time::OffsetDateTime = time::OffsetDateTime::now_utc()
                    + time::Duration::hours(payment_token.retry_wait_time_hours);
                let next_available_time = time::PrimitiveDateTime::new(
                    utc_schedule_time.date(),
                    utc_schedule_time.time(),
                );

                PaymentProcessorTokenResponse::NextAvailableTime {
                    next_available_time,
                }
            } else {
                PaymentProcessorTokenResponse::ScheduledTime { scheduled_time }
            }
        }
    };

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
    let error_code = payment_attempt
        .error
        .as_ref()
        .map(|details| details.code.clone());

    let connector_name = payment_attempt
        .connector
        .clone()
        .ok_or(storage_impl::errors::RecoveryError::ValueNotFound)
        .attach_printable("unable to derive payment connector from payment attempt")?;

    // Stripe returns the same generic `message` for every card decline and carries the issuer's
    // decline code only in `reason` (`message - <message>, decline_code - <decline_code>`), so the
    // gsm lookup uses `reason` for stripe to tell a lost card apart from a retryable decline.
    let matches_gsm_on_error_reason = connector_name
        .parse::<common_enums::connector_enums::Connector>()
        .map(|connector| connector == common_enums::connector_enums::Connector::Stripe)
        .unwrap_or(false);

    let error_message = payment_attempt.error.as_ref().map(|details| {
        matches_gsm_on_error_reason
            .then(|| details.reason.clone())
            .flatten()
            .unwrap_or_else(|| details.message.clone())
    });

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
// Returns `Some(datetime)` whenever the grace window has at least one retriable day, and `None` only
// when the window is empty (`grace_days <= 1` — no future day to retry on). WITHIN a non-empty window
// the pick never fails: Laplace smoothing gives every slot a defined estimate and the runway guard
// guarantees a pick even for sparse/empty stats.
//
// INDEXING (must match `retry_stats_document::EventSlots::from_utc`, which is how the stats are
// recorded):
//   * dow: `weekday().number_days_from_monday()`  →  MONDAY = 0 .. Sunday = 6
//   * dom: `day() - 1`                            →  0-indexed (0 = the 1st)
//   * hod: `hour()`                               →  0 .. 23 (UTC)
// ---------------------------------------------------------------------------

/// Clip C_ij to [CLIP, 1-CLIP] so near-certain comparisons don't send logit to +/- infinity.
#[cfg(feature = "v2")]
const CLIP: f64 = 1e-4;
/// Max candidate-window length. Beyond ~a month the window only repeats weekday/month-day slots that
/// are already represented (all 7 weekdays — and all month-days except when the window spans February),
/// so a longer grace adds little new signal; this caps the work and guards a config typo. NOTE: the cap
/// is a behavior change (a grace > 31 can never propose days 32+), so it's logged where it triggers.
#[cfg(feature = "v2")]
const MAX_GRACE_DAYS: u32 = 31;

/// Laplace-smoothed success rate: p̂ = (k+1)/(n+2). Callers must pass a well-formed counter (k ≤ n);
/// `slot_scores` DROPS corrupt `k > n` slots before this runs, so p̂ ∈ (0,1) strictly and `se` never
/// takes the sqrt of a negative. (Do NOT clamp k→n here — that reads as "all succeeded" and would make
/// a corrupt slot the best in the cluster; dropping degrades it to "no data" instead.)
// `u64 -> f64` has no lossless or checked conversion in std (no `From`/`TryFrom` for floats), so `as`
// is the only option — and it is EXACT for counts up to 2^53 (~9e15), far beyond any real retry tally.
#[cfg(feature = "v2")]
#[allow(clippy::as_conversions)]
fn p_hat(c: SlotCounter) -> f64 {
    (c.k as f64 + 1.0) / (c.n as f64 + 2.0)
}

/// Beta-posterior standard deviation: SE = sqrt(p̂(1-p̂)/(n+3)).
#[cfg(feature = "v2")]
#[allow(clippy::as_conversions)]
fn se(c: SlotCounter) -> f64 {
    let p = p_hat(c);
    (p * (1.0 - p) / (c.n as f64 + 3.0)).sqrt()
}

/// Standard normal cumulative distribution function: `Φ(x) = P(Z ≤ x)` for a standard normal
/// `Z ~ N(0, 1)` — the probability a bell-curve draw falls at or below `x`.
///
/// This is how [`slot_scores`] turns a rate gap into a confidence. Given two slots' Laplace rates
/// (`p̂ᵢ`, `p̂ⱼ`) and standard errors (`SEᵢ`, `SEⱼ`), `Φ((p̂ᵢ − p̂ⱼ) / √(SEᵢ² + SEⱼ²))` is the
/// probability that slot *i*'s TRUE success rate exceeds slot *j*'s — a big, well-separated lead
/// approaches 1, a shaky lead sits near 0.5. So it measures how *confidently* one slot beats
/// another, not just whether its point estimate is higher.
///
/// Uses the standard identity `Φ(x) = ½·(1 + erf(x/√2))` with `libm::erf` — a full-precision,
/// Rust-team-maintained error function (`std` has no stable `erf`).
#[cfg(feature = "v2")]
fn normal_cdf(x: f64) -> f64 {
    0.5 * (1.0 + libm::erf(x / std::f64::consts::SQRT_2))
}

#[cfg(feature = "v2")]
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
#[cfg(feature = "v2")]
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
            "retry_stats: corrupt slot counters (k > n) excluded from scoring"
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

/// Which signal won the softmax `max` for a candidate day. Carried through the pick so the log can
/// name it without re-deriving. `Tie` = both axes equal (e.g. a cold cluster: both uniform).
#[cfg(feature = "v2")]
#[derive(Clone, Copy, Debug)]
enum DayAxis {
    Dow,
    Dom,
    Tie,
}

#[cfg(feature = "v2")]
impl DayAxis {
    fn as_str(self) -> &'static str {
        match self {
            Self::Dow => "dow",
            Self::Dom => "dom",
            Self::Tie => "tie",
        }
    }
}

/// Why `pick_index` landed on the index it returned — so the caller can attribute the pick honestly
/// (a forced or exhausted pick must NOT be logged as if the weights drove it). `Weights` carries the
/// chosen item's tag (for the day pick, its `DayAxis`), resolved AT pick time so the caller never
/// re-indexes to find it.
#[cfg(feature = "v2")]
#[derive(Clone, Copy, Debug)]
enum PickDriver<T> {
    Weights(T),  // the weights drove the choice; carries the chosen item's tag
    RunwayGuard, // forced fire: budget >= remaining candidates (spends the budget before the window ends)
}

#[cfg(feature = "v2")]
impl PickDriver<DayAxis> {
    /// Log label: the winning axis when the weights drove the pick, else the forced-pick reason.
    fn label(self) -> &'static str {
        match self {
            Self::Weights(axis) => axis.as_str(),
            Self::RunwayGuard => "runway_guard",
        }
    }
}

/// Per-tick probabilistic pick over an ordered list of non-negative WEIGHTS, with the runway guard.
/// Fires index k with probability `min(budget · weight_k / remaining_weight, 1)` (remaining_weight
/// via a suffix-sum); `budget >= remaining` forces a fire. Weights need not be normalized — only
/// their ratios matter. `tags` runs parallel to `weights`; the chosen index's tag rides back inside
/// `PickDriver::Weights`, so the caller never re-indexes to recover it. Returns the chosen index and
/// WHY (see `PickDriver`), or `None` when nothing fires — an empty list, or a run where every tick
/// missed. With `budget >= 1` the runway guard forces the last index to fire, so `None` means "no
/// candidate to schedule" (empty list or `budget == 0`); the caller treats it as "don't schedule".
#[cfg(feature = "v2")]
fn pick_index<T: Copy + std::fmt::Debug>(
    weights: &[f64],
    tags: &[T],
    budget: u32,
    context: &'static str,
) -> Option<(usize, PickDriver<T>)> {
    let n = weights.len();
    if n == 0 {
        return None;
    }
    // suffix[k] = weights[k] + … + weights[n-1], accumulated over the reversed slice (no indexing).
    let suffix: Vec<f64> = {
        let mut acc = 0.0;
        let mut sums: Vec<f64> = weights
            .iter()
            .rev()
            .map(|&w| {
                acc += w;
                acc
            })
            .collect();
        sums.reverse();
        sums
    };
    let budget_slots = usize::try_from(budget).unwrap_or(usize::MAX);
    for (k, ((&w, &s), &tag)) in weights
        .iter()
        .zip(suffix.iter())
        .zip(tags.iter())
        .enumerate()
    {
        let remaining = n - k;
        let guard = budget_slots >= remaining; // runway guard forces the fire
        let p = if guard {
            1.0
        } else {
            (f64::from(budget) * w / s).min(1.0)
        };
        // Draw the (unseeded) random value into a variable so the per-step decision is fully logged.
        let draw = rand::random::<f64>();
        let fired = draw < p;
        logger::debug!(
            context = context,
            index = k,
            tag = ?tag,
            weight = w,
            remaining_weight = s,
            guard = guard,
            fire_probability = p,
            rand_draw = draw,
            fired = fired,
            "mathmodel: pick step"
        );
        if fired {
            return Some((
                k,
                if guard {
                    PickDriver::RunwayGuard
                } else {
                    PickDriver::Weights(tag)
                },
            ));
        }
    }
    logger::debug!(
        context = context,
        "mathmodel: pick exhausted — no step fired (budget 0); no candidate to schedule"
    );
    None
}

/// Pick the hour the SAME way as days: per-tick over hours 0..23 with budget 1 (exactly one hour).
/// Missing hours score 0. Falls back to the caller-supplied `default_hour` when there is no USABLE
/// hour data — guard on the SCORED slots, not the raw counters, so all-corrupt counters still fall
/// back (an all-zero array yields empty scores too, so this one check covers all no-data shapes).
#[cfg(feature = "v2")]
fn pick_hour(hod: &[SlotCounter], default_hour: u8) -> u8 {
    let scores = slot_scores(hod);
    if scores.is_empty() {
        return default_hour;
    }
    let hours: Vec<u8> = (0u8..24).collect();
    let weights: Vec<f64> = hours
        .iter()
        .map(|&h| scores.get(&h).copied().unwrap_or(0.0).exp())
        .collect();
    // budget 1 over a non-empty list => the runway guard always fires (never None); the fallback to
    // default_hour is a defensive floor. `hours[idx]` IS the hour (a u8), fetched via `get` — no cast.
    pick_index(&weights, &hours, 1, "hour")
        .and_then(|(idx, _)| hours.get(idx).copied())
        .unwrap_or(default_hour)
}

/// Softmax `xs` (numerically stabilized by subtracting the max). `xs` is non-empty here.
#[cfg(feature = "v2")]
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
#[cfg(feature = "v2")]
fn combine_day_weight(
    dates: &[time::Date],
    dow: &BTreeMap<u8, f64>,
    dom: &BTreeMap<u8, f64>,
) -> (Vec<f64>, Vec<DayAxis>) {
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
    p_dow
        .iter()
        .zip(p_dom.iter())
        .map(|(&pw, &pm)| {
            let winner = if pm > pw {
                DayAxis::Dom
            } else if pw > pm {
                DayAxis::Dow
            } else {
                DayAxis::Tie // both axes equal (e.g. a cold cluster: both uniform) — neither "won"
            };
            (pw.max(pm), winner)
        })
        .unzip()
}

/// Predict the retry datetime from cluster stats.
///
/// * `stats`      the cluster's parsed success stats (dow/dom/hod `{n,k}` counters)
/// * `budget`     retries remaining (drives the runway guard)
/// * `grace_days` grace period, in days, COUNTING the failure day (today). Retriable window = the
///                `grace_days - 1` future days, capped at 31.
/// * `default_hour` fallback hour-of-day (UTC) used when the cluster has no usable hour history
///                (from `revenue_recovery.default_retry_hour_utc` config).
///
/// The window starts on the **NEXT day** (failure day + 1), never on the failure day itself — the
/// charge just failed today, so a same-day retry is low value. The min-gap / past-time guard rails
/// are applied by the CALLER on the returned time (per the MathModel design).
///
/// Returns `None` when `budget == 0` (no retries left to schedule) or `grace_days <= 1` (no future
/// day inside the grace period); otherwise `Some`.
/// V1 LIMITATION: a grace of 1 (today only) with retries still available is treated as "no retry"; a
/// later version will handle that edge (e.g. a same-day retry after a delay). Uses real randomness
/// (no seed). The caller `min()`s the result with the static schedule time.
#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub fn compute_mathmodel_retry_time(
    stats: &StatsDocument,
    budget: u32,
    grace_days: u32,
    default_hour: u8,
) -> Option<time::OffsetDateTime> {
    // `grace_days` COUNTS the failure day (today), which we never retry on — so the retriable window
    // is the `grace_days - 1` future days [failure_day + 1 .. failure_day + grace_days - 1]. When that
    // is empty (grace_days <= 1: no grace, or grace covers only today) there is no in-grace future day.
    // V1: return None here. A later version will handle "grace 1 but retries remain" (e.g. a same-day
    // retry after a delay) rather than skipping.
    let future_days = grace_days.saturating_sub(1);
    if future_days == 0 {
        logger::debug!(
            budget = budget,
            grace_days = grace_days,
            "mathmodel: declined — no future day inside the grace window (grace_days <= 1)"
        );
        return None;
    }
    // No retries remaining: there is nothing to schedule. `pick_index` also returns None on budget 0
    // (nothing fires), so this is really an early-out — it short-circuits before building the window
    // and emitting the per-day trace, and states the "no budget -> no retry" contract explicitly.
    if budget == 0 {
        logger::debug!(
            budget = budget,
            grace_days = grace_days,
            "mathmodel: declined — no retry budget remaining"
        );
        return None;
    }
    let now = time::OffsetDateTime::now_utc();
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
    // future_days is capped at MAX_GRACE_DAYS (31), so this always fits usize; 0 is an unreachable
    // safe floor (would just leave the single seeded `start` day).
    let window_len = usize::try_from(future_days.min(MAX_GRACE_DAYS)).unwrap_or(0);
    let start = now.date().next_day().unwrap_or(now.date());
    let mut dates = vec![start];
    while dates.len() < window_len {
        match dates.last().and_then(|d| d.next_day()) {
            Some(next) => dates.push(next),
            None => break,
        }
    }

    logger::debug!(
        budget = budget,
        grace_days = grace_days,
        window_len = window_len,
        window_start = %start,
        default_hour = default_hour,
        "mathmodel: decision start"
    );

    let (day_weights, winners) = combine_day_weight(&dates, &dow_scores, &dom_scores);

    // Per-candidate-day scores: the day-of-week and day-of-month signals plus the combined weight the
    // pick is about to run on. Zipped (not indexed) over the parallel vectors.
    for ((date, &weight), &winner) in dates.iter().zip(&day_weights).zip(&winners) {
        let dow_score = dow_scores
            .get(&date.weekday().number_days_from_monday())
            .copied()
            .unwrap_or(0.0);
        let dom_score = dom_scores
            .get(&date.day().saturating_sub(1))
            .copied()
            .unwrap_or(0.0);
        logger::debug!(
            date = %date,
            weekday = date.weekday().number_days_from_monday(),
            day_of_month = date.day(),
            dow_score = dow_score,
            dom_score = dom_score,
            combined_weight = weight,
            winning_axis = winner.as_str(),
            "mathmodel: candidate day score"
        );
    }

    // winners ride along so the winning axis returns inside pick_driver — no re-indexing afterwards.
    // `None` means no candidate fired (no budget / empty window) -> nothing to schedule -> return None.
    let (day_idx, pick_driver) = pick_index(&day_weights, &winners, budget, "day")?;
    let hour = pick_hour(&stats.hod, default_hour);
    let time = time::Time::from_hms(hour, 0, 0).unwrap_or(time::Time::MIDNIGHT);

    // day_idx is always in range (from pick_index over these vecs); `.get` keeps it panic-free.
    let chosen_date = *dates.get(day_idx)?;
    let chosen_weight = day_weights.get(day_idx).copied().unwrap_or(0.0);
    let retry_at = chosen_date.with_time(time).assume_offset(now.offset());

    // Final decision. A forced (runway-guard) pick labels itself (not the softmax winner) for honest
    // back-test attribution.
    logger::debug!(
        chosen_day = %chosen_date,
        chosen_hour = hour,
        retry_at = %retry_at,
        driver = pick_driver.label(),
        weight = chosen_weight,
        "mathmodel: decision final"
    );

    Some(retry_at)
}

/// Adaptive retry time for a failed invoice: fetch the cluster's success stats by the
/// (standardised) error code and ask the math model when to retry.
///
/// `remaining_grace_days` and `remaining_budget` are resolved by the caller (from the invoice's
/// grace window and retry allowance) and passed straight through to the model.
///
/// Returns `None` on every "no opinion" case — no stats recorded for the cluster yet, a lookup
/// failure (a corrupt stored key/document surfaces as one), or the model itself declining — so the
/// caller always has the static ladder to fall back on. The returned instant is UTC
/// (`OffsetDateTime`); the codebase stays in explicit UTC and only converts to a naive
/// `PrimitiveDateTime` at the schedule boundary.
#[cfg(feature = "v2")]
pub async fn compute_adaptive_retry_time(
    state: &SessionState,
    error_code: common_enums::StandardisedCode,
    remaining_grace_days: u32,
    remaining_budget: u32,
) -> Option<time::OffsetDateTime> {
    // Fetch the stats recorded against this error code. The store builds the cluster key and
    // parses the stored document internally; `None` when the cluster has no recorded history yet.
    let record = state
        .store
        .get_revenue_recovery_retry_stats_store()
        .find_revenue_recovery_retry_stats_by_error_code(error_code)
        .await
        .map_err(|error| {
            logger::error!(?error, ?error_code, "adaptive retry: failed to fetch stats");
        })
        .ok()??;

    logger::debug!(
        ?error_code,
        remaining_grace_days,
        remaining_budget,
        "adaptive retry: stats fetched — running mathmodel"
    );

    // A configured hour outside 0..=23 is a misconfiguration; warn (so it's visible) and fall back to
    // noon UTC rather than letting it silently degrade to midnight downstream.
    let configured_hour = state.conf.revenue_recovery.default_retry_hour_utc.0;
    let default_hour = if configured_hour <= 23 {
        configured_hour
    } else {
        logger::warn!(
            configured_hour,
            "adaptive retry: revenue_recovery.default_retry_hour_utc is out of range (0-23); using noon UTC"
        );
        12
    };
    compute_mathmodel_retry_time(
        &record.stats,
        remaining_budget,
        remaining_grace_days,
        default_hour,
    )
}

#[cfg(all(test, feature = "v2"))]
mod mathmodel_retry_time_tests {
    use super::*;

    // The default retry hour the production config supplies (see `default_retry_hour_utc`); passed
    // explicitly here so the tests don't depend on config plumbing.
    const DEFAULT_RETRY_HOUR: u8 = 12;

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
            if let Some(slot) = doc.dow.get_mut(i) {
                *slot = ctr(n, k);
            }
        }
        for &(i, n, k) in dom {
            if let Some(slot) = doc.dom.get_mut(i) {
                *slot = ctr(n, k);
            }
        }
        for &(i, n, k) in hod {
            if let Some(slot) = doc.hod.get_mut(i) {
                *slot = ctr(n, k);
            }
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
        let score = |slot: u8| sc.get(&slot).copied().expect("slot was scored");
        assert!(approx(score(4), 9.2103, 0.1), "score_4={}", score(4));
        assert!(approx(score(0), -2.73, 0.1), "score_0={}", score(0));
        assert!(approx(score(6), -6.48, 0.1), "score_6={}", score(6));
    }

    #[test]
    fn combine_maps_scores_to_the_right_weekday() {
        // Exercises the indexing contract (dow index = number_days_from_monday), not just the
        // `time` crate: give Monday (0) a dominant score, no month-day signal, and assert the Monday
        // date in the window wins the weight. Deterministic — combine_day_weight uses no RNG.
        let start = time::Date::from_calendar_date(2026, time::Month::August, 17)
            .expect("valid calendar date"); // a Monday
        let mut dates = vec![start];
        while dates.len() < 7 {
            let next = dates
                .last()
                .and_then(|d| d.next_day())
                .expect("next calendar day exists");
            dates.push(next);
        }
        let dow = BTreeMap::from([(0u8, 5.0)]); // Monday dominant
        let dom = BTreeMap::<u8, f64>::new(); // no month-day signal
        let (weights, _) = combine_day_weight(&dates, &dow, &dom);
        let argmax = weights
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);
        let winning_date = dates.get(argmax).copied().expect("argmax within window");
        assert_eq!(
            winning_date, start,
            "Monday's dominant score must make the Monday date win"
        );
        assert_eq!(winning_date.weekday().number_days_from_monday(), 0);
    }

    #[test]
    fn pick_hour_is_valid_and_empty_defaults() {
        // No seed now, so assert only the invariant: a valid hour; no usable hour data -> default.
        let doc = sample();
        for _ in 0..100 {
            assert!(pick_hour(&doc.hod, DEFAULT_RETRY_HOUR) < 24);
        }
        assert_eq!(
            pick_hour(&[SlotCounter::default(); 24], DEFAULT_RETRY_HOUR),
            DEFAULT_RETRY_HOUR
        );
    }

    #[test]
    fn result_is_always_in_window() {
        // Invariant for EVERY random outcome: result date ∈ [tomorrow, today+grace], hour ∈ 0..23,
        // never panics — for both rich and empty stats. (Window starts on the NEXT day; the bounds
        // carry a 1-day slack so a midnight tick between captures can't flake it.)
        let grace: u32 = 14;
        for stats in [sample(), StatsDocument::default()] {
            for _ in 0..200 {
                let before = time::OffsetDateTime::now_utc();
                let dt = compute_mathmodel_retry_time(&stats, 3, grace, DEFAULT_RETRY_HOUR)
                    .expect("grace > 1 => Some");
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
        let before = time::OffsetDateTime::now_utc();
        let dt = compute_mathmodel_retry_time(&sample(), 3, 2, DEFAULT_RETRY_HOUR)
            .expect("grace 2 => Some");
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
        assert!(compute_mathmodel_retry_time(&sample(), 3, 0, DEFAULT_RETRY_HOUR).is_none());
        assert!(compute_mathmodel_retry_time(&sample(), 3, 1, DEFAULT_RETRY_HOUR).is_none());
    }

    #[test]
    fn zero_budget_returns_none() {
        // No retries left: the model must NOT hand back a date (pick_index with budget 0 would
        // otherwise fall through to the last grace day). Guard holds for any grace / stats shape.
        assert!(compute_mathmodel_retry_time(&sample(), 0, 14, DEFAULT_RETRY_HOUR).is_none());
        assert!(
            compute_mathmodel_retry_time(&StatsDocument::default(), 0, 30, DEFAULT_RETRY_HOUR)
                .is_none()
        );
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
        let before = time::OffsetDateTime::now_utc();
        for _ in 0..50 {
            let dt = compute_mathmodel_retry_time(&corrupt, 3, 14, DEFAULT_RETRY_HOUR)
                .expect("grace > 1 => Some");
            assert!(dt.date() > before.date() && dt.hour() < 24);
        }
    }

    #[test]
    fn corrupt_hod_falls_back_to_default() {
        // All-corrupt hour counters -> no usable scores -> deterministic noon, not a
        // uniform-random hour.
        let doc = doc_with(&[], &[], &[(9, 1, 30)]);
        assert_eq!(pick_hour(&doc.hod, DEFAULT_RETRY_HOUR), DEFAULT_RETRY_HOUR);
    }

    #[test]
    fn grace_is_capped_at_max() {
        let before = time::OffsetDateTime::now_utc();
        let dt = compute_mathmodel_retry_time(&sample(), 3, 365, DEFAULT_RETRY_HOUR)
            .expect("grace > 1 => Some");
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
        let (idx, driver) = pick_index(&[1.0, 1.0, 1.0], &[DayAxis::Tie; 3], 5, "test")
            .expect("guard forces a fire");
        assert_eq!(idx, 0);
        assert!(matches!(driver, PickDriver::RunwayGuard));
    }
}
