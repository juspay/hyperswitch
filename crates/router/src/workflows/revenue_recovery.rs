#[cfg(feature = "v2")]
use std::collections::HashMap;

#[cfg(feature = "v2")]
use api_models::{enums::RevenueRecoveryAlgorithmType, payments::PaymentsGetIntentRequest};
use common_utils::errors::CustomResult;
#[cfg(feature = "v2")]
use common_utils::{
    ext_traits::AsyncExt,
    ext_traits::{StringExt, ValueExt},
    id_type,
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
        let merchant_context_from_revenue_recovery_payment_data =
            domain::MerchantContext::NormalMerchant(Box::new(domain::Context(
                revenue_recovery_payment_data.merchant_account.clone(),
                revenue_recovery_payment_data.key_store.clone(),
            )));
        let (payment_data, _, _) = payments::payments_intent_operation_core::<
            api_types::PaymentGetIntent,
            _,
            _,
            PaymentIntentData<api_types::PaymentGetIntent>,
        >(
            state,
            state.get_req_state(),
            merchant_context_from_revenue_recovery_payment_data.clone(),
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
                    merchant_context_from_revenue_recovery_payment_data.clone(),
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
                    merchant_context_from_revenue_recovery_payment_data.clone(),
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
                    merchant_context_from_revenue_recovery_payment_data,
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

    let key_manager_state = &state.into();
    let key_store = db
        .get_merchant_key_store_by_merchant_id(
            key_manager_state,
            &tracking_data.merchant_id,
            &db.get_master_key().to_vec().into(),
        )
        .await?;

    let merchant_account = db
        .find_merchant_account_by_merchant_id(
            key_manager_state,
            &tracking_data.merchant_id,
            &key_store,
        )
        .await?;

    let profile = db
        .find_business_profile_by_profile_id(
            key_manager_state,
            &key_store,
            &tracking_data.profile_id,
        )
        .await?;

    let billing_mca = db
        .find_merchant_connector_account_by_id(
            key_manager_state,
            &tracking_data.billing_mca_id,
            &key_store,
        )
        .await?;

    let pcr_payment_data = pcr_storage_types::RevenueRecoveryPaymentData {
        merchant_account,
        profile: profile.clone(),
        key_store,
        billing_mca,
        retry_algorithm: profile
            .revenue_recovery_retry_algorithm_type
            .unwrap_or(tracking_data.revenue_recovery_retry),
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

#[cfg(feature = "v2")]
pub(crate) async fn get_schedule_time_for_smart_retry(
    state: &SessionState,
    payment_intent: &PaymentIntent,
    retry_after_time: Option<prost_types::Timestamp>,
    token_with_retry_info: &PaymentProcessorTokenWithRetryInfo,
) -> Result<Option<time::PrimitiveDateTime>, errors::ProcessTrackerError> {
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

    let card_issuer_str = card_info.card_issuer.clone();

    let card_funding_str = match card_info.card_type.as_deref() {
        Some("card") => None,
        Some(s) => Some(s.to_string()),
        None => None,
    };

    let start_time_primitive = payment_intent.created_at;
    let recovery_timestamp_config = &state.conf.revenue_recovery.recovery_timestamp;

    let modified_start_time_primitive = start_time_primitive.saturating_add(time::Duration::hours(
        recovery_timestamp_config.initial_timestamp_in_hours,
    ));

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
        retry_count: Some(
            (total_retry_count_within_network.max_retry_count_for_thirty_day - retry_count_left)
                .into(),
        ),
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
                        Ok(pdt) => Some(pdt),
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
        }
    }
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone)]
pub struct ScheduledToken {
    pub token_details: PaymentProcessorTokenDetails,
    pub schedule_time: time::PrimitiveDateTime,
}

#[cfg(feature = "v2")]
pub async fn get_token_with_schedule_time_based_on_retry_algorithm_type(
    state: &SessionState,
    connector_customer_id: &str,
    payment_intent: &PaymentIntent,
    retry_algorithm_type: RevenueRecoveryAlgorithmType,
    retry_count: i32,
) -> CustomResult<Option<time::PrimitiveDateTime>, errors::ProcessTrackerError> {
    let mut scheduled_time = None;

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

            scheduled_time = Some(time);
        }

        RevenueRecoveryAlgorithmType::Smart => {
            scheduled_time = get_best_psp_token_available_for_smart_retry(
                state,
                connector_customer_id,
                payment_intent,
            )
            .await
            .change_context(errors::ProcessTrackerError::EApiErrorResponse)?;
        }
    }
    let delayed_schedule_time = scheduled_time.map(add_random_delay_to_schedule_time);

    Ok(delayed_schedule_time)
}

#[cfg(feature = "v2")]
pub async fn get_best_psp_token_available_for_smart_retry(
    state: &SessionState,
    connector_customer_id: &str,
    payment_intent: &PaymentIntent,
) -> CustomResult<Option<time::PrimitiveDateTime>, errors::ProcessTrackerError> {
    //  Lock using payment_id
    let locked = RedisTokenManager::lock_connector_customer_status(
        state,
        connector_customer_id,
        &payment_intent.id,
    )
    .await
    .change_context(errors::ProcessTrackerError::ERedisError(
        errors::RedisError::RedisConnectionError.into(),
    ))?;

    match !locked {
        true => Ok(None),

        false => {
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

            // TODO: Insert into payment_intent_feature_metadata (DB operation)

            let result = RedisTokenManager::get_tokens_with_retry_metadata(state, &existing_tokens);

            let best_token_time = call_decider_for_payment_processor_tokens_select_closet_time(
                state,
                &result,
                payment_intent,
                connector_customer_id,
            )
            .await
            .change_context(errors::ProcessTrackerError::EApiErrorResponse)?;

            Ok(best_token_time)
        }
    }
}

#[cfg(feature = "v2")]
pub async fn calculate_smart_retry_time(
    state: &SessionState,
    payment_intent: &PaymentIntent,
    token_with_retry_info: &PaymentProcessorTokenWithRetryInfo,
) -> Result<Option<time::PrimitiveDateTime>, errors::ProcessTrackerError> {
    let wait_hours = token_with_retry_info.retry_wait_time_hours;
    let current_time = time::OffsetDateTime::now_utc();
    let future_time = current_time + time::Duration::hours(wait_hours);

    // Timestamp after which retry can be done without penalty
    let future_timestamp = Some(prost_types::Timestamp {
        seconds: future_time.unix_timestamp(),
        nanos: 0,
    });

    get_schedule_time_for_smart_retry(
        state,
        payment_intent,
        future_timestamp,
        token_with_retry_info,
    )
    .await
}

#[cfg(feature = "v2")]
async fn process_token_for_retry(
    state: &SessionState,
    token_with_retry_info: &PaymentProcessorTokenWithRetryInfo,
    payment_intent: &PaymentIntent,
) -> Result<Option<ScheduledToken>, errors::ProcessTrackerError> {
    let token_status: &PaymentProcessorTokenStatus = &token_with_retry_info.token_status;
    let inserted_by_attempt_id = &token_status.inserted_by_attempt_id;

    let skip = token_status.is_hard_decline.unwrap_or(false);

    match skip {
        true => {
            logger::info!(
                "Skipping decider call due to hard decline token inserted by attempt_id: {}",
                inserted_by_attempt_id.get_string_repr()
            );
            Ok(None)
        }
        false => {
            let schedule_time =
                calculate_smart_retry_time(state, payment_intent, token_with_retry_info).await?;
            Ok(schedule_time.map(|schedule_time| ScheduledToken {
                token_details: token_status.payment_processor_token_details.clone(),
                schedule_time,
            }))
        }
    }
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn call_decider_for_payment_processor_tokens_select_closet_time(
    state: &SessionState,
    processor_tokens: &HashMap<String, PaymentProcessorTokenWithRetryInfo>,
    payment_intent: &PaymentIntent,
    connector_customer_id: &str,
) -> CustomResult<Option<time::PrimitiveDateTime>, errors::ProcessTrackerError> {
    tracing::debug!("Filtered  payment attempts based on payment tokens",);
    let mut tokens_with_schedule_time: Vec<ScheduledToken> = Vec::new();

    for token_with_retry_info in processor_tokens.values() {
        let token_details = &token_with_retry_info
            .token_status
            .payment_processor_token_details;
        let error_code = token_with_retry_info.token_status.error_code.clone();

        match error_code {
            None => {
                let utc_schedule_time =
                    time::OffsetDateTime::now_utc() + time::Duration::minutes(1);

                let schedule_time = time::PrimitiveDateTime::new(
                    utc_schedule_time.date(),
                    utc_schedule_time.time(),
                );
                tokens_with_schedule_time = vec![ScheduledToken {
                    token_details: token_details.clone(),
                    schedule_time,
                }];
                tracing::debug!(
                    "Found payment processor token with no error code scheduling it for {schedule_time}",
                );
                break;
            }
            Some(_) => {
                process_token_for_retry(state, token_with_retry_info, payment_intent)
                    .await?
                    .map(|token_with_schedule_time| {
                        tokens_with_schedule_time.push(token_with_schedule_time)
                    });
            }
        }
    }

    let best_token = tokens_with_schedule_time
        .iter()
        .min_by_key(|token| token.schedule_time)
        .cloned();

    match best_token {
        None => {
            RedisTokenManager::unlock_connector_customer_status(state, connector_customer_id)
                .await
                .change_context(errors::ProcessTrackerError::EApiErrorResponse)?;
            tracing::debug!("No payment processor tokens available for scheduling");
            Ok(None)
        }

        Some(token) => {
            tracing::debug!("Found payment processor token with least schedule time");

            RedisTokenManager::update_payment_processor_token_schedule_time(
                state,
                connector_customer_id,
                &token.token_details.payment_processor_token,
                Some(token.schedule_time),
            )
            .await
            .change_context(errors::ProcessTrackerError::EApiErrorResponse)?;

            Ok(Some(token.schedule_time))
        }
    }
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
    schedule_time: time::PrimitiveDateTime,
) -> time::PrimitiveDateTime {
    let mut rng = rand::thread_rng();
    let random_secs = rng.gen_range(1..=3600);
    logger::info!("Adding random delay of {random_secs} seconds to schedule time");
    schedule_time + time::Duration::seconds(random_secs)
}
