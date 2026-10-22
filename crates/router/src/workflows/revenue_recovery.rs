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
        transformers::ForeignFrom,
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

/// Static ladder time, for the variants that treat the ladder as a ceiling on the model's time.
/// Read at the ladder position rather than the invoice's overall retry count, because
/// `pt_mapping_adaptive_retries` is the mapping built for rung semantics.
#[cfg(feature = "v2")]
pub(crate) async fn get_schedule_time_to_retry_adaptive_payments(
    db: &dyn StorageInterface,
    superposition_client: &external_services::superposition::SuperpositionClient,
    dimensions: &crate::core::configs::dimension_state::DimensionsWithProcessorMerchantIdAndConnector,
    queried_rung: i32,
) -> Option<time::PrimitiveDateTime> {
    let mapping = dimensions
        .get_pt_mapping_adaptive_retries(db, superposition_client, None)
        .await;

    let time_delta = scheduler_utils::get_pcr_payments_retry_schedule_time(mapping, queried_rung);

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
        (common_utils::date_time::now().assume_utc() - most_recent_date.assume_utc()).whole_hours()
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
    let now_utc = common_utils::date_time::now().assume_utc();

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

    /// The invoice is past the end of its recovery grace window, so there is no legitimate time
    /// left to retry at — whatever retry budget remains.
    GraceWindowExpired,

    /// No retry info available / nothing to do yet
    None,
}

#[cfg(feature = "v2")]
impl PaymentProcessorTokenResponse {
    /// Stable metric label for the outcome. Spelled out rather than derived so a renamed variant
    /// cannot silently break the series a dashboard is grouped on.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HardDecline => "hard_decline",
            Self::ScheduledTime { .. } => "scheduled",
            Self::NextAvailableTime { .. } => "next_available_time",
            Self::RetriesExhausted => "retries_exhausted",
            Self::GraceWindowExpired => "grace_window_expired",
            Self::None => "none",
        }
    }
}

/// The assigned arm's retry time for an invoice, or `None` when the model cannot be consulted.
#[cfg(feature = "v2")]
async fn get_retry_time_for_error_code(
    state: &SessionState,
    algorithm: common_enums::RevenueRecoveryABAlgorithm,
    prev_attempt_error_code: Option<common_enums::StandardisedCode>,
    remaining_grace_days: u32,
    remaining_budget: u32,
) -> Option<time::PrimitiveDateTime> {
    let Some(error_code) = prev_attempt_error_code else {
        crate::routes::metrics::REVENUE_RECOVERY_AB_MISSING_ERROR_CODE.add(
            1,
            router_env::metric_attributes!(("algorithm", algorithm.to_string())),
        );
        logger::info!(
            ?algorithm,
            "retry model: the CALCULATE task carries no error code, so the model cannot be \
             consulted — falling back to the cascading ladder"
        );
        return None;
    };

    compute_model_retry_time(
        state,
        error_code,
        remaining_grace_days,
        remaining_budget,
        RetryModelVariant::from(algorithm),
    )
    .await
    .map(common_utils::date_time::convert_to_pdt)
}

/// Picks the retry time for an invoice enrolled in A/B routing, then finds a token for it.
#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
async fn get_ab_routed_retry_time(
    state: &SessionState,
    payment_intent: &PaymentIntent,
    tracking_data: &pcr_storage_types::RevenueRecoveryWorkflowTrackingData,
    remaining_grace_days: u32,
    remaining_budget: u32,
    // Both needed only to assign an implementation to an invoice that arrives without one
    revenue_recovery_payment_data: &pcr_storage_types::RevenueRecoveryPaymentData,
    ab_dimensions: &crate::core::configs::dimension_state::DimensionsWithProcessorAndProviderMerchantIdAndOrgIdAndProfileId,
) -> (
    Option<time::PrimitiveDateTime>,
    common_enums::RevenueRecoveryABAlgorithm,
) {
    let algorithm = payment_intent
        .feature_metadata
        .as_ref()
        .and_then(|feature_metadata| feature_metadata.payment_revenue_recovery_metadata.as_ref())
        .and_then(|revenue_recovery_metadata| {
            revenue_recovery_metadata.revenue_recovery_ab_routing
        });

    logger::info!(
        payment_id = %payment_intent.id.get_string_repr(),
        ?algorithm,
        "A/B routing read the retry implementation assigned to this invoice"
    );

    // Resolve the arm first, then run it once. Dispatching inside the arms would leave the
    // just-assigned invoice on a second path, and an arm added to only one of them would run the
    // wrong algorithm on an invoice's FIRST retry and the right one thereafter.
    let assigned_algorithm = match algorithm {
        Some(assigned_algorithm) => assigned_algorithm,
        None => {
            crate::routes::metrics::REVENUE_RECOVERY_AB_UNASSIGNED_ALGORITHM.add(1, &[]);

            pcr::assign_and_record_ab_routing(
                state,
                &payment_intent.id,
                payment_intent
                    .feature_metadata
                    .as_ref()
                    .map(api_models::payments::FeatureMetadata::foreign_from),
                revenue_recovery_payment_data,
                ab_dimensions,
            )
            .await
        }
    };

    let schedule_time = get_retry_time_for_error_code(
        state,
        assigned_algorithm,
        tracking_data.prev_attempt_error_code,
        remaining_grace_days,
        remaining_budget,
    )
    .await;

    logger::info!(
        ?assigned_algorithm,
        schedule_time = ?schedule_time,
        "A/B routing ran the invoice's assigned arm"
    );

    (schedule_time, assigned_algorithm)
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn get_token_with_schedule_time_based_on_retry_algorithm_type(
    state: &SessionState,
    connector_customer_id: &str,
    payment_intent: &PaymentIntent,
    billing_connector: common_enums::connector_enums::Connector,
    retry_algorithm_type: RevenueRecoveryAlgorithmType,
    retry_count: i32,
    tracking_data: &pcr_storage_types::RevenueRecoveryWorkflowTrackingData,
    static_ladder_progress: &pcr::schedule::StaticLadderProgress,
    // Needed only to resolve the A/B gate
    provider_merchant_id: hyperswitch_domain_models::platform::ProviderMerchantId,
    remaining_grace_days: u32,
    remaining_budget: u32,
    // Needed only to assign an implementation to an invoice that arrives without one
    revenue_recovery_payment_data: &pcr_storage_types::RevenueRecoveryPaymentData,
) -> CustomResult<
    (
        PaymentProcessorTokenResponse,
        Option<pcr::schedule::StaticLadderProgress>,
    ),
    errors::ProcessTrackerError,
> {
    let mut payment_processor_token_response = PaymentProcessorTokenResponse::None;
    // Ladder position to write back onto the CALCULATE row. `None` leaves the stored count alone,
    // which is the case for every outcome but one — see where it is set.
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

            let ab_dimensions = crate::core::configs::dimension_state::Dimensions::new()
                .with_processor_merchant_id(payment_intent.merchant_id.clone().into())
                .with_provider_merchant_id(provider_merchant_id)
                .with_organization_id(payment_intent.organization_id.clone())
                .with_profile_id(payment_intent.profile_id.clone());

            let ab_enabled = ab_dimensions
                .get_revenue_recovery_ab_enabled(
                    state.store.as_ref(),
                    state.superposition_service.as_ref(),
                    None,
                )
                .await;

            if ab_enabled || adaptive_retry_enabled {
                // Same shape as the cascading arm — compute the schedule time, then gate on the
                // token. The addition is the model's candidate, and the variant says both how it
                // is produced and whether the static ladder bounds it; the cascading ladder covers
                // only what every source in play declines.
                //
                // Enrolled and unenrolled invoices differ ONLY in that variant. The allowances it
                // works against arrive as arguments, and everything after it — the fallback, the
                // decision, the token — is shared, so enrolling an invoice cannot change whether
                // it gets retried at all.
                let (model_time, assigned_algorithm) = if ab_enabled {
                    let (time, algorithm) = get_ab_routed_retry_time(
                        state,
                        payment_intent,
                        tracking_data,
                        remaining_grace_days,
                        remaining_budget,
                        revenue_recovery_payment_data,
                        &ab_dimensions,
                    )
                    .await;
                    (time, Some(algorithm))
                } else {
                    let time = match tracking_data.prev_attempt_error_code {
                        // Not enrolled in A/B routing, so this runs the baseline pairing.
                        Some(error_code) => compute_model_retry_time(
                            state,
                            error_code,
                            remaining_grace_days,
                            remaining_budget,
                            RetryModelVariant::default(),
                        )
                        .await
                        .map(common_utils::date_time::convert_to_pdt),
                        // Counted here as well as on the A/B branch: the condition is the same,
                        // and with A/B off this is the branch every invoice takes, so counting it
                        // only there would read zero exactly when it matters.
                        None => {
                            crate::routes::metrics::REVENUE_RECOVERY_AB_MISSING_ERROR_CODE.add(
                                1,
                                router_env::metric_attributes!(("algorithm", "unenrolled")),
                            );
                            logger::info!(
                                payment_id = %payment_intent.id.get_string_repr(),
                                "retry model: the CALCULATE task carries no error code, so the \
                                 model cannot be consulted — falling back to the cascading \
                                 ladder"
                            );
                            None
                        }
                    };
                    (time, None)
                };

                let ladder_role = assigned_algorithm.map_or_else(
                    || RetryModelVariant::default().ladder,
                    |arm| RetryModelVariant::from(arm).ladder,
                );
                let queried_rung = static_ladder_progress.next_rung();

                // Read only where it can win, and only while the merchant's configured length
                // still has the rung being asked for. Rungs run 1..=max.
                let static_time = match ladder_role {
                    pcr::schedule::StaticLadderRole::Standby => None,
                    pcr::schedule::StaticLadderRole::Ceiling => {
                        let max_hybrid_cascading_retry_count = revenue_recovery_payment_data
                            .billing_mca
                            .get_max_hybrid_cascading_retry_count()
                            .map(i32::from)
                            .ok_or(errors::ProcessTrackerError::MissingRequiredField)
                            .attach_printable(
                                "Failed to get max hybrid cascading retry count from billing merchant connector account",
                            )?;

                        if queried_rung <= max_hybrid_cascading_retry_count {
                            get_schedule_time_to_retry_adaptive_payments(
                                state.store.as_ref(),
                                state.superposition_service.as_ref(),
                                &dimensions,
                                queried_rung,
                            )
                            .await
                        } else {
                            logger::info!(
                                queried_rung,
                                max_hybrid_cascading_retry_count,
                                "hybrid ladder spent for this invoice — the merchant's configured \
                                 length is reached, so the model's time stands unbounded"
                            );
                            None
                        }
                    }
                };

                // The global fallback for everything the sources above decline, indexed by the
                // invoice's overall retry count. `None` means "not consulted", not "nothing left".
                let fallback_time = match (model_time, static_time) {
                    (None, None) => {
                        get_schedule_time_to_retry_mit_payments(
                            state.store.as_ref(),
                            state.superposition_service.as_ref(),
                            &dimensions,
                            retry_count,
                        )
                        .await
                    }
                    _ => None,
                };

                let decision = pcr::schedule::decide_next_retry(
                    static_ladder_progress,
                    ladder_role,
                    queried_rung,
                    model_time,
                    static_time,
                    fallback_time,
                )
                .ok_or_else(|| {
                    logger::error!(
                        retry_count = retry_count,
                        queried_rung = queried_rung,
                        ladder_role = ?ladder_role,
                        error_code = ?tracking_data.prev_attempt_error_code,
                        remaining_grace_days = remaining_grace_days,
                        remaining_budget = remaining_budget,
                        "No retry time available — every source in play declined and the MIT \
                         cascading ladder had nothing left"
                    );
                    // Counted, not just logged: an arm that loses invoices at a different rate
                    // from another is measuring its own drop rate rather than retry quality, and
                    // an aggregate count cannot show that.
                    crate::routes::metrics::REVENUE_RECOVERY_NO_SCHEDULE_TIME.add(
                        1,
                        router_env::metric_attributes!((
                            "algorithm",
                            assigned_algorithm
                                .map_or_else(|| "unenrolled".to_string(), |arm| arm.to_string())
                        )),
                    );
                    errors::ProcessTrackerError::FlowExecutionError {
                        flow: "revenue_recovery_no_schedule_time",
                    }
                })?;

                logger::info!(
                    source = ?decision.source,
                    retry_count = retry_count,
                    ladder_role = ?ladder_role,
                    queried_rung = queried_rung,
                    model_time = ?model_time,
                    static_time = ?static_time,
                    fallback_time = ?fallback_time,
                    consumed_rungs = decision.next_progress.consumed_rungs,
                    error_code = ?tracking_data.prev_attempt_error_code,
                    remaining_grace_days = remaining_grace_days,
                    remaining_budget = remaining_budget,
                    ab_enabled = ab_enabled,
                    schedule_time = ?decision.schedule_time,
                    "Retry decision"
                );

                payment_processor_token_response = get_token_availability_for_schedule_time(
                    state,
                    connector_customer_id,
                    payment_intent,
                    decision.schedule_time,
                )
                .await?;

                // The ladder position is carried back only when a retry is genuinely scheduled.
                // The other responses finish or reschedule the CALCULATE job without an attempt.
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

        PaymentProcessorTokenResponse::GraceWindowExpired => {
            logger::debug!("Grace window expired");
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
/// Shared by the cascading and model paths so both gate on the same conditions
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
                let utc_schedule_time: time::OffsetDateTime = common_utils::date_time::now()
                    .assume_utc()
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
    let current_time = common_utils::date_time::now().assume_utc();
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
            common_utils::date_time::now().assume_utc() + time::Duration::seconds(schedule_offset);
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

            let utc_schedule_time =
                common_utils::date_time::now().assume_utc() + time::Duration::minutes(1);
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
    let delay_limit = state
        .conf
        .revenue_recovery
        .recovery_timestamp
        .max_random_schedule_delay_in_seconds;
    let random_secs = common_utils::generate_random_number_in_range(1, delay_limit);
    logger::info!("Adding random delay of {random_secs} seconds to schedule time");
    schedule_time + time::Duration::seconds(random_secs)
}

// ---------------------------------------------------------------------------
// Retry-time prediction — the data-driven half of the Cascading strategy.
//
// Given a cluster's day-of-week / day-of-month / hour-of-day success stats (`StatsDocument`), the
// remaining retry budget, and the grace window, it returns the datetime to retry on. The caller
// takes this time as it stands; the MIT cascading ladder covers only the decisions this
// declines.
//
// The DAY is produced by two independently selectable stages — see `RetryModelVariant`: a COMBINE
// folding the weekday and month-day signals into one weight per candidate day, and a SELECTION
// drawing one day from those weights. The HOUR always uses the per-tick walk.
//
// Returns `Some(datetime)` whenever the grace window has at least one retriable day, and `None` only
// when the window is empty (`grace_days <= 1` — no future day to retry on). WITHIN a non-empty window
// the pick never fails: Laplace smoothing gives every slot a defined estimate, and both samplers
// always land on a day — the runway guard forces one for per-tick, and `Σπ = budget ≥ 1` puts at
// least one probe on the line for systematic-k.
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

/// How the day-of-week and day-of-month signals are folded into one weight per candidate day.
/// Both take a `max` of the two axes; they differ in whether the axes are rescaled first.
///
/// Softmaxing each axis before the max makes each one sum to 1 across the candidate days, so the
/// two are comparable whatever their raw spread. Taking the max first skips that, so the axis
/// carrying larger raw scores wins on scale alone.
#[cfg(feature = "v2")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DayCombine {
    /// Softmax each axis over the candidate days, then take the elementwise max.
    MaxAtSoftmax,
    /// Take the elementwise max of the raw scores, then softmax once.
    MaxAtScore,
}

/// How one day is drawn from the per-day weights.
#[cfg(feature = "v2")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DaySelection {
    /// Per-day Bernoulli walk with the runway guard. Consumes the weights' MAGNITUDES.
    PerTick,
    /// Systematic πps: draw a whole schedule, act on its soonest day. Rank-only — magnitudes are
    /// discarded, so any monotone transform of the weights yields an identical draw.
    SystematicK,
}

/// Which combine, which sampler and what the static ladder is to the result. Independent axes on
/// purpose: a combine change, a sampler change and a scheduling-rule change are separately
/// attributable only if they can be varied separately.
///
/// Public because the arm an invoice gets is an experiment-assignment decision, which belongs above
/// this layer — this module only executes the variant it is handed. `Default` is the baseline an
/// invoice runs when nothing selects otherwise; it is NOT the same as the control arm, which is
/// pinned separately in the `From` impl below.
#[cfg(feature = "v2")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetryModelVariant {
    pub combine: DayCombine,
    pub selection: DaySelection,
    pub ladder: pcr::schedule::StaticLadderRole,
}

/// The pairing production ran before systematic sampling landed, which is what an invoice outside
/// the experiment gets. Not the control arm, which is pinned separately.
#[cfg(feature = "v2")]
impl Default for RetryModelVariant {
    fn default() -> Self {
        Self {
            combine: DayCombine::MaxAtSoftmax,
            selection: DaySelection::PerTick,
            ladder: pcr::schedule::StaticLadderRole::Ceiling,
        }
    }
}

/// The experiment arm an invoice was assigned, resolved to the pair of choices that actually run.
///
/// This is the only place the experiment taxonomy meets the retry model, which is why the match is
/// exhaustive with no `_` arm: a new arm added upstream must fail to compile here rather than fall
/// through to some other pairing and quietly produce an experiment that measures nothing.
///
/// Every arm spells its pairing out rather than deferring to `Default`. An arm is persisted on the
/// invoice and replayed across its retries, so one that tracked the default would switch algorithm
/// mid-recovery the moment the default moved.
#[cfg(feature = "v2")]
impl From<common_enums::RevenueRecoveryABAlgorithm> for RetryModelVariant {
    fn from(algorithm: common_enums::RevenueRecoveryABAlgorithm) -> Self {
        match algorithm {
            common_enums::RevenueRecoveryABAlgorithm::AdaptiveRetry => Self {
                combine: DayCombine::MaxAtSoftmax,
                selection: DaySelection::PerTick,
                ladder: pcr::schedule::StaticLadderRole::Standby,
            },
            common_enums::RevenueRecoveryABAlgorithm::HybridAdaptiveRetry => Self {
                combine: DayCombine::MaxAtSoftmax,
                selection: DaySelection::PerTick,
                ladder: pcr::schedule::StaticLadderRole::Ceiling,
            },
            common_enums::RevenueRecoveryABAlgorithm::SystematicKMaxAtSoftmax => Self {
                combine: DayCombine::MaxAtSoftmax,
                selection: DaySelection::SystematicK,
                ladder: pcr::schedule::StaticLadderRole::Standby,
            },
            common_enums::RevenueRecoveryABAlgorithm::SystematicKMaxAtScore => Self {
                combine: DayCombine::MaxAtScore,
                selection: DaySelection::SystematicK,
                ladder: pcr::schedule::StaticLadderRole::Standby,
            },
        }
    }
}

/// Which signal won the `max` for a candidate day. Carried through the pick so the log can name it
/// without re-deriving. `Tie` = both axes equal (e.g. a cold cluster: both uniform).
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
/// chosen item's tag, resolved AT pick time so the caller never re-indexes to find it.
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
/// Serves the HOUR axis; the day axis draws its whole schedule at once via `select_systematic_k_day`.
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
        // Draw into a variable so the per-step decision is fully logged.
        let draw = common_utils::generate_random_f64_unit();
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
            "retry model: pick step"
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
        "retry model: pick exhausted — no step fired (budget 0); no candidate to schedule"
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
        // Without this the chosen hour is indistinguishable from one the model actually picked,
        // since a real pick can land on the default hour too.
        logger::debug!(
            default_hour = default_hour,
            "retry model: no usable hour-of-day history — falling back to the configured hour"
        );
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
/// Both variants say "this day is good if either its weekday OR its month-day is historically good";
/// they differ in whether the two axes are put on a common footing before the `max` (see
/// [`DayCombine`]). Shared trade-off: `max` optimism — a day strong on one axis but weak on the other
/// is picked on its strong side. A better combine (sum-of-logits, posterior sampling) changes ONLY
/// this function.
///
/// The softmax is kept in both arms even though `SystematicK` discards magnitudes, because
/// `PerTick` does not: the weights must be well-formed for whichever sampler runs.
///
/// Returns per-day `(weight, winning_axis)`; the winner lets the caller log which signal drove a pick.
/// Note the winner is decided on whatever the `max` compared, so it is NOT comparable across
/// variants — the same day can report a different axis under each.
#[cfg(feature = "v2")]
fn combine_day_weight(
    dates: &[time::Date],
    dow: &BTreeMap<u8, f64>,
    dom: &BTreeMap<u8, f64>,
    combine: DayCombine,
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

    // Whichever pair the `max` compares — normalized per axis, or raw — decides both the weight and
    // the attributed axis.
    let (weekday_values, month_day_values) = match combine {
        DayCombine::MaxAtSoftmax => (softmax(&dow_sc), softmax(&dom_sc)),
        DayCombine::MaxAtScore => (dow_sc, dom_sc),
    };

    let (maxed, winners): (Vec<f64>, Vec<DayAxis>) = weekday_values
        .iter()
        .zip(month_day_values.iter())
        .map(|(&weekday, &month_day)| {
            let winner = if month_day > weekday {
                DayAxis::Dom
            } else if weekday > month_day {
                DayAxis::Dow
            } else {
                DayAxis::Tie // both axes equal (e.g. a cold cluster: both uniform) — neither "won"
            };
            (weekday.max(month_day), winner)
        })
        .unzip();

    let weights = match combine {
        DayCombine::MaxAtSoftmax => maxed, // already per-axis probabilities
        DayCombine::MaxAtScore => softmax(&maxed),
    };
    (weights, winners)
}

/// Per-day values → inclusion probabilities. Exact optimum of
///
/// ```text
///   maximise   Σ πᵢ·vᵢ      (spend the budget on the best days)
///   subject to Σ πᵢ = k     (spend it exactly)
///              πᵢ ≤ 1       (a day cannot be retried twice)
///              πᵢ ≥ ε       (every day stays reachable)
/// ```
///
/// The optimum is a greedy pour, no solver: floor every day at ε, then raise days to 1.0 in
/// descending value order until the budget runs out.
///
/// Two guards below fail SILENTLY if removed — the vector still sums to k and still yields k days:
///  * **ε capped at k/n.** Above it the problem is infeasible and the pour emits NEGATIVE
///    probabilities, breaking the monotone cumulative sum `systematic_sample` binary-searches.
///  * **Days within `tie_tolerance` split their share.** Ranking alone breaks ties by array
///    position, giving days of equal value wildly different probabilities. Splitting leaves the
///    group total untouched.
#[cfg(feature = "v2")]
fn inclusion_probabilities(
    values: &[f64],
    budget: u32,
    epsilon: f64,
    tie_tolerance: f64,
) -> Vec<f64> {
    let n = values.len();
    if n == 0 {
        return Vec::new();
    }
    // `usize -> f64`. std offers no `From`/`TryFrom` here (a 64-bit `usize` exceeds f64's exact
    // range), and routing through `u32` would be WORSE than the cast: the fallback for an
    // out-of-range count returns a wrong magnitude, where `as` stays exact to 2^53 and merely
    // rounds beyond it. `n` is a candidate-day count, capped at `MAX_GRACE_DAYS` (31).
    #[allow(clippy::as_conversions)]
    let n_f = n as f64;
    // πᵢ ≤ 1 caps the total at n, so a budget wider than the window can only spend n — every day
    // pinned at 1.0, which is the old runway guard arrived at by arithmetic rather than by a rule.
    let k = f64::from(budget).min(n_f);
    let eps = epsilon.clamp(0.0, k / n_f);

    let mut pi = vec![eps; n];
    let mut remaining = k - eps * n_f;

    // (day index, value), best first. Carrying the value means neither the sort nor the grouping
    // below looks anything up again. `total_cmp` orders floats outright where `partial_cmp` has no
    // answer for NaN; `.reverse()` makes it descending.
    let mut ranked: Vec<(usize, f64)> = values.iter().copied().enumerate().collect();
    ranked.sort_by(|(_, value), (_, other_value)| value.total_cmp(other_value).reverse());

    for &(day, _) in &ranked {
        if remaining <= 1e-12 {
            break;
        }
        if let Some(slot) = pi.get_mut(day) {
            let take = (1.0 - eps).min(remaining);
            *slot += take;
            remaining -= take;
        }
    }

    // Split each tied group's share equally over its members (see the doc comment above). `ranked`
    // is sorted by value, so tied days are adjacent and `chunk_by` hands each run over as a slice.
    for tied_days in
        ranked.chunk_by(|(_, value), (_, other_value)| (value - other_value).abs() <= tie_tolerance)
    {
        if tied_days.len() > 1 {
            let total: f64 = tied_days.iter().filter_map(|&(day, _)| pi.get(day)).sum();
            // `usize -> f64`, exact: a tied group is a subset of the window, so at most 31.
            #[allow(clippy::as_conversions)]
            let share = total / tied_days.len() as f64;
            for &(day, _) in tied_days {
                if let Some(slot) = pi.get_mut(day) {
                    *slot = share;
                }
            }
        }
    }
    pi
}

/// Pick exactly k distinct days, each with probability exactly `pi[i]` — one uniform draw, one pass.
///
/// Lay the probabilities end to end on a line. They sum to k, so the line is exactly k units long;
/// take probes at `u, u+1, … u+(k−1)` and keep whichever day's segment each probe lands in.
///
/// Both guarantees are geometric, not statistical — they hold on every single draw, not on average:
///  * **exactly k distinct days** — the last probe sits at `u+k−1 < k`, so all k land on the line;
///    probes are exactly 1 apart and no segment exceeds length 1 (πᵢ ≤ 1), so no two share a day.
///  * **each day at exactly πᵢ** — wrap the line onto a circle of circumference 1 and all k probes
///    map to the same point, namely `u`, which is uniform; a segment of length πᵢ therefore catches
///    a probe with probability exactly πᵢ.
///
/// `order` lays the segments down shuffled, REDRAWN PER CALL. It changes neither guarantee (both
/// come from segment lengths) but it decides which day COMBINATIONS are reachable: in calendar
/// order a day at π = 1.00 fills a whole unit and is caught wherever `u` starts, collapsing the
/// joint distribution to a handful of schedules with many day-pairs at probability zero. A shuffle
/// computed once and reused is just a different fixed order and fixes nothing.
///
/// Numerical limit: accumulated edges drift ~1e-14, so for `u` within that of 1.0 the last probe
/// can fall past the final edge and collapse onto a taken segment, yielding k−1 days. Needs
/// `u > 1 − 1e-14` and the caller uses only the soonest day, so it is documented rather than
/// patched — the fixes for it (rescaling probes, normalising π) are what silently break G1/G2.
///
/// `probe_count` is `k`, passed rather than recovered from `Σπ`. The two are equal by construction
/// — `inclusion_probabilities` pours exactly `k` — but reading it back off the accumulated line
/// means a float round-trip, and an infinite or absurd `pi` would then produce a probe count to
/// match and loop on it. Being told how many probes to place removes that and the cast with it.
#[cfg(feature = "v2")]
fn systematic_sample(pi: &[f64], u: f64, order: &[usize], probe_count: usize) -> Vec<usize> {
    // Right edge of each segment, accumulated along the shuffled line.
    let mut acc = 0.0;
    let edges: Vec<f64> = order
        .iter()
        .map(|&idx| {
            acc += pi.get(idx).copied().unwrap_or(0.0);
            acc
        })
        .collect();
    if edges.is_empty() {
        return Vec::new();
    }

    let mut selected: Vec<usize> = (0..probe_count)
        .filter_map(|step| {
            // `usize -> f64`, exact: `step` is bounded by `probe_count`, itself the budget.
            // Written `u + step` rather than an accumulated `+ 1.0` per probe: one rounding
            // instead of `step` of them, and the doc above records why probe arithmetic is
            // left alone.
            #[allow(clippy::as_conversions)]
            let probe = u + step as f64;
            // First segment whose right edge is strictly past the probe.
            let position = edges
                .partition_point(|&edge| edge <= probe)
                .min(edges.len().saturating_sub(1));
            order.get(position).copied()
        })
        .collect();
    // Calendar order, so the caller's "soonest" is the first. G1 already makes the days distinct;
    // the dedup covers the clamp above firing on a probe pushed past the last edge by drift.
    selected.sort_unstable();
    selected.dedup();
    selected
}

/// Choose which candidate day to schedule: draw a whole systematic-k schedule over the window, then
/// take the NEAREST day it selected.
///
/// The alternative to the per-day walk (`pick_index`), which decides a day at a time without seeing
/// the end of the window and so skips good days early, runs out of slack and hits the runway guard.
/// That guard is not patchable: at `days_remaining == budget_remaining` the constraints have one
/// feasible point. Deciding how MANY days before WHICH days makes the state unreachable, since
/// `Σπ = k` comes from the geometry of the line rather than a countdown.
///
/// Values enter only through the ranking, so magnitudes are discarded — deliberate, given the
/// upstream softmax turns small quality differences into enormous weight ratios.
///
/// NOTE — the ON-DEMAND form: the schedule is redrawn each decision and only its earliest day used.
/// Drawing once at first failure and persisting all k dates is the recommended design; on-demand
/// drifts from its promised marginals and starves the end of the window. What it costs is
/// measurability, not recovery. Upfront needs somewhere to persist the k dates and their πᵢ.
///
/// Returns the chosen index into `weights` together with its inclusion probability.
#[cfg(feature = "v2")]
fn select_systematic_k_day(
    weights: &[f64],
    budget: u32,
    epsilon: f64,
    tie_tolerance: f64,
) -> Option<(usize, f64)> {
    if weights.is_empty() || budget == 0 {
        return None;
    }
    let pi = inclusion_probabilities(weights, budget, epsilon, tie_tolerance);

    // Redrawn per invoice — see `systematic_sample` on why a cached order fixes nothing.
    let order = common_utils::generate_random_permutation(weights.len());

    let u = common_utils::generate_random_f64_unit();
    // The same `k` the pour spends: the budget, capped by the window it has to spend it over.
    let probe_count = usize::try_from(budget)
        .unwrap_or(usize::MAX)
        .min(weights.len());
    let selected = systematic_sample(&pi, u, &order, probe_count);
    let chosen = selected.first().copied()?;
    let chosen_inclusion_probability = pi.get(chosen).copied().unwrap_or(0.0);

    logger::debug!(
        budget = budget,
        epsilon = epsilon,
        window_len = weights.len(),
        uniform_draw = u,
        scheduled_days = ?selected,
        chosen_index = chosen,
        chosen_inclusion_probability = chosen_inclusion_probability,
        "retry model: systematic-k selection"
    );

    Some((chosen, chosen_inclusion_probability))
}

/// Predict the retry datetime from cluster stats.
///
/// * `stats`      the cluster's parsed success stats (dow/dom/hod `{n,k}` counters)
/// * `budget`     retries remaining (the per-tick runway guard, and the k of systematic-k)
/// * `grace_days` grace period, in days, COUNTING the failure day (today). Retriable window = the
///                `grace_days - 1` future days, capped at 31.
/// * `default_hour` fallback hour-of-day (UTC) used when the cluster has no usable hour history
///                (from `revenue_recovery.default_retry_hour_utc` config).
///
/// The window starts on the **NEXT day** (failure day + 1), never on the failure day itself — the
/// charge just failed today, so a same-day retry is low value. That is also what keeps the result
/// in the future: nothing downstream re-checks it, and the only post-processing is a few seconds of
/// random jitter.
///
/// Returns `None` when `budget == 0` (no retries left to schedule) or `grace_days <= 1` (no future
/// day inside the grace period); otherwise `Some`.
/// V1 LIMITATION: a grace of 1 (today only) with retries still available is treated as "no retry"; a
/// later version will handle that edge (e.g. a same-day retry after a delay). The caller takes the
/// result as it stands, rather than bounding it by the cascading ladder.
#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub fn compute_predicted_retry_time(
    stats: &StatsDocument,
    budget: u32,
    grace_days: u32,
    default_hour: u8,
    variant: RetryModelVariant,
    exploration_floor: f64,
    tie_tolerance: f64,
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
            "retry model: declined — no future day inside the grace window (grace_days <= 1)"
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
            "retry model: declined — no retry budget remaining"
        );
        return None;
    }
    let now = common_utils::date_time::now().assume_utc();
    let dow_scores = slot_scores(&stats.dow);
    let dom_scores = slot_scores(&stats.dom);

    // Cap the window at MAX_GRACE_DAYS: beyond ~a month it only repeats already-covered weekday/
    // month-day slots. Truncation is a behavior change (days beyond the cap are never proposed); log
    // at debug, not warn — it fires per invoice for a mis-configured profile and warn would spam.
    if future_days > MAX_GRACE_DAYS {
        logger::debug!(
            configured = grace_days,
            capped = MAX_GRACE_DAYS,
            "retry model: grace window capped"
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
        exploration_floor = exploration_floor,
        tie_tolerance = tie_tolerance,
        combine = ?variant.combine,
        selection = ?variant.selection,
        "retry model: decision start"
    );

    let (day_weights, winners) =
        combine_day_weight(&dates, &dow_scores, &dom_scores, variant.combine);

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
            "retry model: candidate day score"
        );
    }

    // `None` means there was nothing to schedule (no budget / empty window) -> return None.
    // `driver` names what settled the pick; `inclusion_probability` is only defined for the sampler
    // that computes one.
    let (day_idx, driver, day_inclusion_probability) = match variant.selection {
        DaySelection::PerTick => {
            let (idx, pick_driver) = pick_index(&day_weights, &winners, budget, "day")?;
            (idx, pick_driver.label(), None)
        }
        DaySelection::SystematicK => {
            let (idx, pi) =
                select_systematic_k_day(&day_weights, budget, exploration_floor, tie_tolerance)?;
            let axis = winners.get(idx).map_or("unknown", |axis| axis.as_str());
            (idx, axis, Some(pi))
        }
    };
    let hour = pick_hour(&stats.hod, default_hour);
    let time = time::Time::from_hms(hour, 0, 0).unwrap_or(time::Time::MIDNIGHT);

    // day_idx is always in range (both samplers index these vecs); `.get` keeps it panic-free.
    let chosen_date = *dates.get(day_idx)?;
    let chosen_weight = day_weights.get(day_idx).copied().unwrap_or(0.0);
    let retry_at = chosen_date.with_time(time).assume_offset(now.offset());

    // Final decision. The inclusion probability is logged because it is the propensity this draw was
    // made under: without it the schedule cannot be evaluated off-policy afterwards, and it cannot be
    // reconstructed at analysis time because the value surface drifts between draw and analysis.
    logger::debug!(
        chosen_day = %chosen_date,
        chosen_hour = hour,
        retry_at = %retry_at,
        combine = ?variant.combine,
        selection = ?variant.selection,
        driver = driver,
        inclusion_probability = ?day_inclusion_probability,
        weight = chosen_weight,
        "retry model: decision final"
    );

    Some(retry_at)
}

/// Retry time for a failed invoice: fetch the cluster's success stats by the (standardised) error
/// code and ask the assigned retry model when to retry.
///
/// `remaining_grace_days` and `remaining_budget` are resolved by the caller (from the invoice's
/// grace window and retry allowance) and passed straight through to the model.
///
/// Returns `None` on every "no opinion" case — no stats recorded for the cluster yet, a lookup
/// failure (a corrupt stored key/document surfaces as one), or the model itself declining — so the
/// caller always has the global fallback behind it. The returned instant is UTC
/// (`OffsetDateTime`); the codebase stays in explicit UTC and only converts to a naive
/// `PrimitiveDateTime` at the schedule boundary.
#[cfg(feature = "v2")]
pub async fn compute_model_retry_time(
    state: &SessionState,
    error_code: common_enums::StandardisedCode,
    remaining_grace_days: u32,
    remaining_budget: u32,
    variant: RetryModelVariant,
) -> Option<time::OffsetDateTime> {
    // The store builds the cluster key and parses the stored document internally. A missing cluster
    // and a failed lookup both decline, but they are different problems, so they are logged apart —
    // a cluster with no history yet is the ordinary reason this path produces no time.
    let record = match state
        .store
        .get_revenue_recovery_retry_stats_store()
        .find_revenue_recovery_retry_stats_by_error_code(error_code)
        .await
    {
        Ok(Some(record)) => record,
        Ok(None) => {
            logger::info!(
                ?error_code,
                remaining_grace_days,
                remaining_budget,
                "retry model: no stats recorded for this cluster yet — declining"
            );
            return None;
        }
        Err(error) => {
            logger::error!(?error, ?error_code, "retry model: failed to fetch stats");
            // Counted apart from the cold-start decline above: that one is expected and shrinks as
            // clusters accumulate history, this one is the store failing and does not.
            crate::routes::metrics::REVENUE_RECOVERY_STATS_LOOKUP_FAILED.add(
                1,
                router_env::metric_attributes!(("cluster", error_code.to_string())),
            );
            return None;
        }
    };

    logger::debug!(
        ?error_code,
        remaining_grace_days,
        remaining_budget,
        "retry model: stats fetched — running the model"
    );

    // Each tunable validates itself and warns on the way past, so a misconfigured deployment is
    // visible rather than silently degraded. See the `resolve` methods for what each rejects.
    let retry_time = compute_predicted_retry_time(
        &record.stats,
        remaining_budget,
        remaining_grace_days,
        state.conf.revenue_recovery.default_retry_hour_utc.resolve(),
        variant,
        state.conf.revenue_recovery.exploration_floor.resolve(),
        state.conf.revenue_recovery.tie_tolerance.resolve(),
    );

    // Every reason the model itself declines (empty grace window, spent budget, a sampler that
    // fired nothing) is logged at debug inside the model, which is off in production. Log the
    // decline once here at info so the rate is visible; the reason stays at debug.
    if retry_time.is_none() {
        logger::info!(
            ?error_code,
            remaining_grace_days,
            remaining_budget,
            combine = ?variant.combine,
            selection = ?variant.selection,
            "retry model: declined to schedule — falling back to the cascading ladder"
        );
    }

    retry_time
}

// Split rather than `cfg(all(test, feature = "v2"))`: clippy recognises a test module by a literal
// `cfg(test)` attribute and does not look inside `all(...)`, so the combined form loses the
// in-test lint allowances (`expect_used`, `unwrap_used`, `panic`) that `.clippy.toml` grants.
#[cfg(test)]
#[cfg(feature = "v2")]
mod retry_model_tests {
    use super::*;

    // The values the production config supplies, passed explicitly so the tests do not depend on
    // config plumbing. Kept in step with the defaults on `RevenueRecoverySettings`.
    const DEFAULT_RETRY_HOUR: u8 = 12;
    const EXPLORATION_FLOOR: f64 = 0.10;
    const TIE_TOLERANCE: f64 = 1e-4;

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

    // Every combine x selection x ladder-role triple. The contract tests below run over all eight,
    // because the guarantees they assert (in-window, never panics, declines only when it should)
    // must not depend on which variant an invoice was bucketed into.
    fn all_variants() -> [RetryModelVariant; 8] {
        use pcr::schedule::StaticLadderRole::{Ceiling, Standby};
        use DayCombine::{MaxAtScore, MaxAtSoftmax};
        use DaySelection::{PerTick, SystematicK};

        [
            (MaxAtSoftmax, PerTick, Standby),
            (MaxAtSoftmax, PerTick, Ceiling),
            (MaxAtSoftmax, SystematicK, Standby),
            (MaxAtSoftmax, SystematicK, Ceiling),
            (MaxAtScore, PerTick, Standby),
            (MaxAtScore, PerTick, Ceiling),
            (MaxAtScore, SystematicK, Standby),
            (MaxAtScore, SystematicK, Ceiling),
        ]
        .map(|(combine, selection, ladder)| RetryModelVariant {
            combine,
            selection,
            ladder,
        })
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
        for combine in [DayCombine::MaxAtSoftmax, DayCombine::MaxAtScore] {
            let (weights, _) = combine_day_weight(&dates, &dow, &dom, combine);
            let argmax = weights
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i)
                .unwrap_or(0);
            let winning_date = dates.get(argmax).copied().expect("argmax within window");
            assert_eq!(
                winning_date, start,
                "{combine:?}: Monday's dominant score must make the Monday date win"
            );
            assert_eq!(winning_date.weekday().number_days_from_monday(), 0);
        }
    }

    fn window_from(year: i32, month: time::Month, day: u8, len: usize) -> Vec<time::Date> {
        let start = time::Date::from_calendar_date(year, month, day).expect("valid calendar date");
        let mut dates = vec![start];
        while dates.len() < len {
            let next = dates
                .last()
                .and_then(|d| d.next_day())
                .expect("next calendar day exists");
            dates.push(next);
        }
        dates
    }

    #[test]
    fn combines_disagree_because_recurrence_dilutes_the_weekday_axis() {
        // The two combines differ only by a per-axis offset, and under MaxAtSoftmax that offset
        // carries a structural term: softmaxing the weekday axis OVER THE CANDIDATE DAYS splits a
        // weekday's mass across its recurrences, while a day-of-month occurs once in any window of
        // <= 31 days and keeps all of its. So the same evidence ranks differently, and the gap grows
        // with the window. 2026-08-25 + 14 days holds two Fridays and one 1st.
        let dates = window_from(2026, time::Month::August, 25, 14);
        let dow = BTreeMap::from([(4u8, 9.21)]); // Friday, decisive within its axis
        let dom = BTreeMap::from([(0u8, 5.09)]); // the 1st, a narrower lead within its axis
        let top = |w: &[f64]| {
            w.iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i)
                .unwrap_or(0)
        };

        let (a_weights, _) = combine_day_weight(&dates, &dow, &dom, DayCombine::MaxAtSoftmax);
        let (b_weights, _) = combine_day_weight(&dates, &dow, &dom, DayCombine::MaxAtScore);
        let a_top = dates.get(top(&a_weights)).copied().expect("top in window");
        let b_top = dates.get(top(&b_weights)).copied().expect("top in window");

        assert_eq!(a_top.day(), 1, "MaxAtSoftmax should favour the unique 1st");
        assert_eq!(
            b_top.weekday().number_days_from_monday(),
            4,
            "MaxAtScore compares raw scores, so the higher-scoring Friday should win"
        );
        assert_ne!(
            a_top, b_top,
            "the two combines must be distinguishable here"
        );
    }

    #[test]
    fn empty_axis_floors_the_ranking_under_max_at_softmax() {
        // COLD START. An axis with no recorded history scores 0.0 everywhere, and under
        // MaxAtSoftmax that softmaxes to a UNIFORM 1/n — which the `max` then applies as a floor.
        // Any preference the live axis expresses below 1/n is erased: here the Thursdays (score 2.0)
        // collapse onto days with no signal at all, leaving two distinct weights where the raw scores
        // have three. Under systematic-k those flattened days form one tie group and split their
        // probability equally, so the live axis's ordering among them is gone, not merely compressed.
        // MaxAtScore floors at a raw 0.0 instead and keeps the ordering.
        let dates = window_from(2026, time::Month::August, 25, 14);
        let dow_only = BTreeMap::from([(0u8, 5.0), (3u8, 2.0)]); // Monday strong, Thursday mild
        let empty = BTreeMap::<u8, f64>::new();
        let (a_weights, _) =
            combine_day_weight(&dates, &dow_only, &empty, DayCombine::MaxAtSoftmax);
        let (b_weights, _) = combine_day_weight(&dates, &dow_only, &empty, DayCombine::MaxAtScore);

        // index 2 = Thursday 27 Aug (score 2.0), index 0 = Tuesday 25 Aug (no signal).
        let a_thursday = a_weights.get(2).copied().expect("index in window");
        let a_unscored = a_weights.first().copied().expect("index in window");
        let b_thursday = b_weights.get(2).copied().expect("index in window");
        let b_unscored = b_weights.first().copied().expect("index in window");

        assert!(
            approx(a_thursday, a_unscored, 1e-12),
            "MaxAtSoftmax should floor both at the uniform 1/n: {a_thursday} vs {a_unscored}"
        );
        assert!(
            b_thursday > b_unscored,
            "MaxAtScore should keep the Thursday above an unscored day: {b_thursday} vs {b_unscored}"
        );

        let distinct = |w: &[f64]| {
            let mut v: Vec<u64> = w.iter().map(|x| x.to_bits()).collect();
            v.sort_unstable();
            v.dedup();
            v.len()
        };
        assert_eq!(distinct(&a_weights), 2, "A flattens to two levels");
        assert_eq!(distinct(&b_weights), 3, "B keeps all three levels");
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
        for variant in all_variants() {
            for stats in [sample(), StatsDocument::default()] {
                for _ in 0..200 {
                    let before = common_utils::date_time::now().assume_utc();
                    let dt = compute_predicted_retry_time(
                        &stats,
                        3,
                        grace,
                        DEFAULT_RETRY_HOUR,
                        variant,
                        EXPLORATION_FLOOR,
                        TIE_TOLERANCE,
                    )
                    .expect("grace > 1 => Some");
                    let last = (before + time::Duration::days(i64::from(grace) + 1)).date();
                    assert!(
                        dt.date() > before.date() && dt.date() <= last,
                        "{variant:?}: date {} out of window",
                        dt.date()
                    );
                    assert!(dt.hour() < 24);
                }
            }
        }
    }

    #[test]
    fn window_starts_next_day() {
        // Failure day is excluded: the earliest candidate is tomorrow. grace COUNTS today, so grace 2
        // = today + 1 future day (tomorrow) — assert the pick is that next day, not the failure day.
        for variant in all_variants() {
            let before = common_utils::date_time::now().assume_utc();
            let dt = compute_predicted_retry_time(
                &sample(),
                3,
                2,
                DEFAULT_RETRY_HOUR,
                variant,
                EXPLORATION_FLOOR,
                TIE_TOLERANCE,
            )
            .expect("grace 2 => Some");
            assert!(
                dt.date() > before.date(),
                "{variant:?}: expected next day, got {} (today {})",
                dt.date(),
                before.date()
            );
            assert!(dt.date() <= (before + time::Duration::days(2)).date());
        }
    }

    #[test]
    fn grace_zero_and_one_return_none() {
        // grace COUNTS today; grace 0 = no grace, grace 1 = today only -> no future day -> None (v1).
        for variant in all_variants() {
            assert!(compute_predicted_retry_time(
                &sample(),
                3,
                0,
                DEFAULT_RETRY_HOUR,
                variant,
                EXPLORATION_FLOOR,
                TIE_TOLERANCE
            )
            .is_none());
            assert!(compute_predicted_retry_time(
                &sample(),
                3,
                1,
                DEFAULT_RETRY_HOUR,
                variant,
                EXPLORATION_FLOOR,
                TIE_TOLERANCE
            )
            .is_none());
        }
    }

    #[test]
    fn zero_budget_returns_none() {
        // No retries left: the model must NOT hand back a date (pick_index with budget 0 would
        // otherwise fall through to the last grace day). Guard holds for any grace / stats shape.
        for variant in all_variants() {
            assert!(compute_predicted_retry_time(
                &sample(),
                0,
                14,
                DEFAULT_RETRY_HOUR,
                variant,
                EXPLORATION_FLOOR,
                TIE_TOLERANCE
            )
            .is_none());
            assert!(compute_predicted_retry_time(
                &StatsDocument::default(),
                0,
                30,
                DEFAULT_RETRY_HOUR,
                variant,
                EXPLORATION_FLOOR,
                TIE_TOLERANCE,
            )
            .is_none());
        }
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
        let before = common_utils::date_time::now().assume_utc();
        for _ in 0..50 {
            let dt = compute_predicted_retry_time(
                &corrupt,
                3,
                14,
                DEFAULT_RETRY_HOUR,
                RetryModelVariant::default(),
                EXPLORATION_FLOOR,
                TIE_TOLERANCE,
            )
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
        let before = common_utils::date_time::now().assume_utc();
        let dt = compute_predicted_retry_time(
            &sample(),
            3,
            365,
            DEFAULT_RETRY_HOUR,
            RetryModelVariant::default(),
            EXPLORATION_FLOOR,
            TIE_TOLERANCE,
        )
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

    // A window of distinct descending values, so the greedy pour has an unambiguous ranking.
    fn descending_values(n: usize) -> Vec<f64> {
        let mut value = 1.0;
        (0..n)
            .map(|_| {
                let current = value;
                value -= 0.01;
                current
            })
            .collect()
    }

    #[test]
    fn inclusion_probabilities_spend_the_budget_exactly() {
        // The line must be exactly k units long and every segment a usable probability.
        // `expected` is written out, not recomputed, so the cap is asserted rather than mirrored.
        for (n, budget, eps, expected) in [
            (30, 15, 0.10, 15.0),
            (30, 15, 0.00, 15.0),
            (7, 3, 0.25, 3.0),
            (31, 1, 0.10, 1.0),
            (5, 5, 0.10, 5.0),
            (5, 9, 0.10, 5.0), // budget wider than the window: capped at n
        ] {
            let pi = inclusion_probabilities(&descending_values(n), budget, eps, TIE_TOLERANCE);
            let total: f64 = pi.iter().sum();
            assert!(
                approx(total, expected, 1e-9),
                "n={n} budget={budget} eps={eps}: Σπ={total}, expected {expected}"
            );
            assert!(
                pi.iter().all(|&p| (0.0..=1.0).contains(&p)),
                "n={n} budget={budget} eps={eps}: π outside [0,1]: {pi:?}"
            );
        }
    }

    #[test]
    fn epsilon_above_k_over_n_is_capped_not_trusted() {
        // ε > k/n makes the remainder negative, so the pour emits negative probabilities while
        // still summing to k and returning k days — nothing notices, but the cumsum stops rising.
        let n = 30;
        let budget = 15; // k/n = 0.5
        for eps in [0.55, 0.60, 0.95] {
            let pi = inclusion_probabilities(&descending_values(n), budget, eps, TIE_TOLERANCE);
            let min = pi.iter().copied().fold(f64::MAX, f64::min);
            assert!(
                min >= 0.0,
                "eps={eps} produced a negative probability: {min}"
            );
            assert!(approx(pi.iter().sum::<f64>(), f64::from(budget), 1e-9));
        }
    }

    #[test]
    fn tied_values_receive_equal_probability() {
        // Ranking alone breaks ties by array position. Equal evidence must mean equal odds.
        let values = vec![0.9, 0.1809, 0.1809, 0.1809, 0.1809, 0.05];
        let pi = inclusion_probabilities(&values, 3, 0.10, TIE_TOLERANCE);
        let tied = pi.get(1..5).expect("tied group within range");
        let first = tied.first().copied().expect("non-empty tied group");
        assert!(
            tied.iter().all(|&p| approx(p, first, 1e-12)),
            "tied days drew unequal probabilities: {tied:?}"
        );
        assert!(approx(pi.iter().sum::<f64>(), 3.0, 1e-9));
    }

    #[test]
    fn tie_tolerance_merges_only_within_its_own_width() {
        // Two days straddling the rank the pour runs out at — the only place merging changes
        // anything. Both directions asserted: swallowing the second would reallocate probability
        // between days the model does rank apart.
        let inside = vec![0.5 + TIE_TOLERANCE / 10.0, 0.5, 0.2, 0.1];
        let pi = inclusion_probabilities(&inside, 2, 0.10, TIE_TOLERANCE);
        let first = pi.first().copied().expect("index in range");
        let second = pi.get(1).copied().expect("index in range");
        assert!(
            approx(first, second, 1e-12),
            "values within the tolerance must split: {first} vs {second}"
        );

        let outside = vec![0.5 + TIE_TOLERANCE * 1000.0, 0.5, 0.2, 0.1];
        let pi = inclusion_probabilities(&outside, 2, 0.10, TIE_TOLERANCE);
        let first = pi.first().copied().expect("index in range");
        let second = pi.get(1).copied().expect("index in range");
        assert!(
            first > second,
            "values outside the tolerance must keep their order: {first} vs {second}"
        );

        // Splitting redistributes inside the group, so the budget is spent exactly either way.
        for values in [inside, outside] {
            let total: f64 = inclusion_probabilities(&values, 2, 0.10, TIE_TOLERANCE)
                .iter()
                .sum();
            assert!(
                approx(total, 2.0, 1e-9),
                "budget not spent exactly: {total}"
            );
        }
    }

    #[test]
    fn control_arm_is_pinned_and_does_not_track_the_default() {
        // AdaptiveRetry is persisted on live invoices, so it must keep its pairing whatever the
        // default becomes — otherwise an in-flight invoice switches algorithm mid-recovery.
        assert_eq!(
            RetryModelVariant::from(common_enums::RevenueRecoveryABAlgorithm::AdaptiveRetry),
            RetryModelVariant {
                combine: DayCombine::MaxAtSoftmax,
                selection: DaySelection::PerTick,
                ladder: pcr::schedule::StaticLadderRole::Standby,
            }
        );
    }

    #[test]
    fn the_unenrolled_default_is_the_pairing_production_ran_before_systematic_sampling() {
        // This `Default` is the only thing selecting an unenrolled invoice's algorithm, so moving
        // it re-points every invoice outside the experiment.
        assert_eq!(
            RetryModelVariant::default(),
            RetryModelVariant {
                combine: DayCombine::MaxAtSoftmax,
                selection: DaySelection::PerTick,
                ladder: pcr::schedule::StaticLadderRole::Ceiling,
            }
        );
    }

    #[test]
    fn each_arm_varies_at_most_one_axis_from_its_reference() {
        // Attributability: SystematicKMaxAtSoftmax moves only the sampler from control,
        // SystematicKMaxAtScore only the combine from it, and HybridAdaptiveRetry only the
        // ladder's role. An arm moving two axes leaves nothing able to say which change caused
        // a result.
        use common_enums::RevenueRecoveryABAlgorithm as Arm;
        let control = RetryModelVariant::from(Arm::AdaptiveRetry);
        let hybrid = RetryModelVariant::from(Arm::HybridAdaptiveRetry);
        let softmax_k = RetryModelVariant::from(Arm::SystematicKMaxAtSoftmax);
        let score_k = RetryModelVariant::from(Arm::SystematicKMaxAtScore);

        let axes_differing = |a: RetryModelVariant, b: RetryModelVariant| {
            usize::from(a.combine != b.combine)
                + usize::from(a.selection != b.selection)
                + usize::from(a.ladder != b.ladder)
        };
        assert_eq!(
            axes_differing(control, softmax_k),
            1,
            "sampler comparison must hold the combine and the ladder fixed"
        );
        assert_eq!(
            axes_differing(softmax_k, score_k),
            1,
            "combine comparison must hold the sampler and the ladder fixed"
        );
        assert_eq!(
            axes_differing(control, hybrid),
            1,
            "ladder comparison must hold the combine and the sampler fixed"
        );
    }

    #[test]
    fn arms_map_to_distinct_variants() {
        // Two arms on the same pairing compare an arm against itself, which reads as a null.
        use strum::IntoEnumIterator;
        let mut seen: Vec<RetryModelVariant> = Vec::new();
        for arm in common_enums::RevenueRecoveryABAlgorithm::iter() {
            let variant = RetryModelVariant::from(arm);
            assert!(
                !seen.contains(&variant),
                "{arm:?} duplicates an earlier arm's variant: {variant:?}"
            );
            seen.push(variant);
        }
    }

    #[test]
    fn arm_config_strings_are_pinned() {
        // These are the values configured in the Superposition workspace, derived by strum rather
        // than written out — the trailing K is the kind of thing a derive can render either way.
        use common_enums::RevenueRecoveryABAlgorithm as Arm;
        for (arm, expected) in [
            (Arm::AdaptiveRetry, "adaptive_retry"),
            (Arm::HybridAdaptiveRetry, "hybrid_adaptive_retry"),
            (Arm::SystematicKMaxAtSoftmax, "systematic_k_max_at_softmax"),
            (Arm::SystematicKMaxAtScore, "systematic_k_max_at_score"),
        ] {
            assert_eq!(arm.to_string(), expected);
            assert_eq!(
                expected.parse::<Arm>().expect("config string must parse"),
                arm,
                "a config value that does not round-trip falls back to the configured default, \
                 silently enrolling the invoice in an arm nobody chose"
            );
        }
    }

    #[test]
    fn entropy_seams_match_what_the_sampler_assumes() {
        // Two `common_utils` contracts the exactly-k guarantee rests on and the type system does
        // not enforce, pinned here so a seam change fails in this module.

        // u ∈ [0, 1): at 1.0 the last probe sits at k, off the end of a k-unit line. Probed
        // statistically, so this catches a widened range, not the exact endpoint.
        for _ in 0..10_000 {
            let u = common_utils::generate_random_f64_unit();
            assert!((0.0..1.0).contains(&u), "u outside [0, 1): {u}");
        }

        // `order` addresses the line, so a duplicate would lay one day down twice and drop
        // another, breaking both the marginals and the distinct-days guarantee.
        for length in [1usize, 2, 7, 29, 31] {
            let order = common_utils::generate_random_permutation(length);
            assert_eq!(order.len(), length, "wrong length for {length}");
            let mut sorted = order.clone();
            sorted.sort_unstable();
            assert!(
                sorted.iter().copied().eq(0..length),
                "not a permutation of 0..{length}: {order:?}"
            );
        }
    }

    #[test]
    fn systematic_sample_always_returns_exactly_k_distinct_days() {
        // G1 is geometric, so it holds on EVERY draw, not on average — including the endpoints
        // u = 0 and u → 1⁻ where probes sit exactly on segment boundaries.
        let n = 30;
        let budget: u32 = 15;
        let pi = inclusion_probabilities(&descending_values(n), budget, 0.10, TIE_TOLERANCE);
        let order: Vec<usize> = (0..n).collect();
        let expected_days = usize::try_from(budget).unwrap_or(0);
        for step in 0..1000 {
            let u = f64::from(step) / 1000.0;
            let selected = systematic_sample(&pi, u, &order, expected_days);
            assert_eq!(
                selected.len(),
                expected_days,
                "u={u} selected {} days, expected {budget}",
                selected.len()
            );
            let mut distinct = selected.clone();
            distinct.dedup();
            assert_eq!(distinct.len(), selected.len(), "u={u} repeated a day");
        }
    }

    #[test]
    fn realised_selection_rate_matches_the_promised_probability() {
        // G2: day i is selected with probability exactly πᵢ. Checked as a frequency over draws, so
        // the tolerance is sampling noise on 20k draws, not the guarantee's own error.
        let n = 12;
        let values = descending_values(n);
        let pi = inclusion_probabilities(&values, 4, 0.10, TIE_TOLERANCE);
        let order: Vec<usize> = (0..n).collect();
        let draws: u32 = 20_000;
        let mut hits = vec![0u32; n];
        for step in 0..draws {
            let u = (f64::from(step) + 0.5) / f64::from(draws);
            for day in systematic_sample(&pi, u, &order, 4) {
                if let Some(count) = hits.get_mut(day) {
                    *count += 1;
                }
            }
        }
        for (day, (&count, &promised)) in hits.iter().zip(pi.iter()).enumerate() {
            let realised = f64::from(count) / f64::from(draws);
            assert!(
                approx(realised, promised, 0.01),
                "day {day}: realised {realised}, promised {promised}"
            );
        }
    }

    #[test]
    fn exploration_floor_keeps_every_day_reachable_in_the_schedule() {
        // What ε buys: every day lands in SOME schedule, including the worst-rated. Asserted on
        // the draw, since which day the caller acts on is a different question — see below.
        let n = 10;
        let pi = inclusion_probabilities(&descending_values(n), 3, 0.10, TIE_TOLERANCE);
        let order: Vec<usize> = (0..n).collect();
        let mut seen = vec![false; n];
        for step in 0..2_000 {
            let u = (f64::from(step) + 0.5) / 2000.0;
            for day in systematic_sample(&pi, u, &order, 3) {
                if let Some(hit) = seen.get_mut(day) {
                    *hit = true;
                }
            }
        }
        assert!(
            seen.iter().all(|&hit| hit),
            "a day was never reachable despite the exploration floor: {seen:?}"
        );
    }

    #[test]
    fn returned_day_is_the_nearest_one_drawn() {
        // On-demand returns the SOONEST day, so a day pinned at π = 1.0 near the front comes back
        // every time: ε spreads the schedule, not the day acted on. Front-loaded values make the
        // pick deterministic; only back-loaded ones let it move.
        let front_best = descending_values(10);
        for _ in 0..200 {
            let (idx, pi) =
                select_systematic_k_day(&front_best, 3, 0.10, TIE_TOLERANCE).expect("budget > 0");
            assert_eq!(
                idx, 0,
                "a pinned soonest day must always be the one returned"
            );
            assert!(approx(pi, 1.0, 1e-9), "day 0 should be pinned, got π={pi}");
        }

        let back_best: Vec<f64> = descending_values(10).into_iter().rev().collect();
        let mut distinct = std::collections::BTreeSet::new();
        for _ in 0..400 {
            let (idx, _) =
                select_systematic_k_day(&back_best, 3, 0.10, TIE_TOLERANCE).expect("budget > 0");
            distinct.insert(idx);
        }
        assert!(
            distinct.len() > 1,
            "back-loaded values should not pin the returned day: {distinct:?}"
        );
    }

    #[test]
    fn systematic_k_day_declines_only_when_there_is_nothing_to_schedule() {
        assert!(select_systematic_k_day(&[], 5, 0.10, TIE_TOLERANCE).is_none());
        assert!(select_systematic_k_day(&[1.0, 1.0], 0, 0.10, TIE_TOLERANCE).is_none());
        // Budget wider than the window pins every day at 1.0, so the soonest day is always day 0.
        let (idx, pi) =
            select_systematic_k_day(&[0.1, 0.2, 0.3], 9, 0.10, TIE_TOLERANCE).expect("budget > 0");
        assert_eq!(idx, 0);
        assert!(
            approx(pi, 1.0, 1e-9),
            "every day should be certain, got {pi}"
        );
    }
}
