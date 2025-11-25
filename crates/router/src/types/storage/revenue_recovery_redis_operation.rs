use std::collections::HashMap;

use api_models::revenue_recovery_data_backfill::{self, AccountUpdateHistoryRecord, RedisKeyType};
use common_enums::enums::CardNetwork;
use common_utils::{date_time, errors::CustomResult, id_type};
use error_stack::ResultExt;
use masking::{ExposeInterface, PeekInterface, Secret};
use redis_interface::{DelReply, SetnxReply};
use router_env::{instrument, logger, tracing};
use serde::{Deserialize, Deserializer, Serialize};
use time::{Date, Duration, OffsetDateTime, PrimitiveDateTime, Time};

use crate::{db::errors, types::storage::enums::RevenueRecoveryAlgorithmType, SessionState};

// Constants for retry window management
const INITIAL_RETRY_COUNT: i32 = 0;
const RETRY_WINDOW_IN_HOUR: i32 = 720;

/// Payment processor token details including card information
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct PaymentProcessorTokenDetails {
    pub payment_processor_token: String,
    pub expiry_month: Option<Secret<String>>,
    pub expiry_year: Option<Secret<String>>,
    pub card_issuer: Option<String>,
    pub last_four_digits: Option<String>,
    pub card_network: Option<CardNetwork>,
    pub card_type: Option<String>,
    pub card_isin: Option<String>,
}

/// Represents the status and retry history of a payment processor token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentProcessorTokenStatus {
    /// Payment processor token details including card information and token ID
    pub payment_processor_token_details: PaymentProcessorTokenDetails,
    /// Payment intent ID that originally inserted this token
    pub inserted_by_attempt_id: id_type::GlobalAttemptId,
    /// Error code associated with the token failure
    pub error_code: Option<String>,
    /// Daily retry count history for the last 30 days (date -> retry_count)
    #[serde(deserialize_with = "parse_datetime_key")]
    pub daily_retry_history: HashMap<PrimitiveDateTime, i32>,
    /// Scheduled time for the next retry attempt
    pub scheduled_at: Option<PrimitiveDateTime>,
    /// Indicates if the token is a hard decline (no retries allowed)
    pub is_hard_decline: Option<bool>,
    /// Timestamp of the last modification to this token status
    pub modified_at: Option<PrimitiveDateTime>,
    /// Indicates if the token is active or not
    pub is_active: Option<bool>,
    /// Update history of the token
    pub account_update_history: Option<Vec<AccountUpdateHistoryRecord>>,
    /// Previous Decision threshold for selecting the best slot
    pub decision_threshold: Option<f64>,
}

impl From<&PaymentProcessorTokenDetails> for api_models::payments::AdditionalCardInfo {
    fn from(data: &PaymentProcessorTokenDetails) -> Self {
        Self {
            card_exp_month: data.expiry_month.clone(),
            card_exp_year: data.expiry_year.clone(),
            card_issuer: data.card_issuer.clone(),
            card_network: data.card_network.clone(),
            card_type: data.card_type.clone(),
            last4: data.last_four_digits.clone(),
            card_isin: data.card_isin.clone(),
            card_issuing_country: None,
            bank_code: None,
            card_extended_bin: None,
            card_holder_name: None,
            payment_checks: None,
            authentication_data: None,
            is_regulated: None,
            signature_network: None,
        }
    }
}

fn parse_datetime_key<'de, D>(deserializer: D) -> Result<HashMap<PrimitiveDateTime, i32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw: HashMap<String, i32> = HashMap::deserialize(deserializer)?;
    let mut parsed = HashMap::new();

    // Full datetime
    let full_dt_format = time::macros::format_description!(
        "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]"
    );
    // Date only
    let date_only_format = time::macros::format_description!("[year]-[month]-[day]");

    for (k, v) in raw {
        let dt = PrimitiveDateTime::parse(&k, &full_dt_format)
            .or_else(|_| {
                Date::parse(&k, &date_only_format)
                    .map(|date| PrimitiveDateTime::new(date, Time::MIDNIGHT))
            })
            .map_err(|_| serde::de::Error::custom(format!("Invalid date key: {}", k)))?;

        parsed.insert(dt, v);
    }

    Ok(parsed)
}

/// Token retry availability information with detailed wait times
#[derive(Debug, Clone)]
pub struct TokenRetryInfo {
    pub monthly_wait_hours: i64,   // Hours to wait for 30-day limit reset
    pub daily_wait_hours: i64,     // Hours to wait for daily limit reset
    pub total_30_day_retries: i32, // Current total retry count in 30-day window
}

/// Complete token information with retry limits and wait times
#[derive(Debug, Clone)]
pub struct PaymentProcessorTokenWithRetryInfo {
    /// The complete token status information
    pub token_status: PaymentProcessorTokenStatus,
    /// Hours to wait before next retry attempt (max of daily/monthly wait)
    pub retry_wait_time_hours: i64,
    /// Number of retries remaining in the 30-day rolling window
    pub monthly_retry_remaining: i32,
    // Current total retry count in 30-day window
    pub total_30_day_retries: i32,
}

/// Redis-based token management struct
pub struct RedisTokenManager;

impl RedisTokenManager {
    fn get_connector_customer_lock_key(connector_customer_id: &str) -> String {
        format!("customer:{connector_customer_id}:status")
    }

    fn get_connector_customer_tokens_key(connector_customer_id: &str) -> String {
        format!("customer:{connector_customer_id}:tokens")
    }

    /// Lock connector customer
    #[instrument(skip_all)]
    pub async fn lock_connector_customer_status(
        state: &SessionState,
        connector_customer_id: &str,
        payment_id: &id_type::GlobalPaymentId,
    ) -> CustomResult<bool, errors::StorageError> {
        let redis_conn =
            state
                .store
                .get_redis_conn()
                .change_context(errors::StorageError::RedisError(
                    errors::RedisError::RedisConnectionError.into(),
                ))?;

        let lock_key = Self::get_connector_customer_lock_key(connector_customer_id);
        let seconds = &state.conf.revenue_recovery.redis_ttl_in_seconds;

        let result: bool = match redis_conn
            .set_key_if_not_exists_with_expiry(
                &lock_key.into(),
                payment_id.get_string_repr(),
                Some(*seconds),
            )
            .await
        {
            Ok(resp) => resp == SetnxReply::KeySet,
            Err(error) => {
                tracing::error!(operation = "lock_stream", err = ?error);
                false
            }
        };

        tracing::debug!(
            connector_customer_id = connector_customer_id,
            payment_id = payment_id.get_string_repr(),
            lock_acquired = %result,
            "Connector customer lock attempt"
        );

        Ok(result)
    }
    #[instrument(skip_all)]
    pub async fn update_connector_customer_lock_ttl(
        state: &SessionState,
        connector_customer_id: &str,
        exp_in_seconds: i64,
    ) -> CustomResult<(), errors::StorageError> {
        let redis_conn =
            state
                .store
                .get_redis_conn()
                .change_context(errors::StorageError::RedisError(
                    errors::RedisError::RedisConnectionError.into(),
                ))?;

        let lock_key = Self::get_connector_customer_lock_key(connector_customer_id);

        let result: bool = redis_conn
            .set_expiry(&lock_key.clone().into(), exp_in_seconds)
            .await
            .map_or_else(
                |error| {
                    tracing::error!(operation = "update_lock_ttl", err = ?error);
                    false
                },
                |_| true,
            );

        if result {
            tracing::debug!(
                lock_key = %lock_key,
                new_ttl_in_seconds = exp_in_seconds,
                "Redis key TTL updated successfully"
            );
        } else {
            tracing::error!(
                lock_key = %lock_key,
                "Failed to update TTL: key not found or error occurred"
            );
        }

        Ok(())
    }

    /// Unlock connector customer status
    #[instrument(skip_all)]
    pub async fn unlock_connector_customer_status(
        state: &SessionState,
        connector_customer_id: &str,
        payment_id: &id_type::GlobalPaymentId,
    ) -> CustomResult<bool, errors::StorageError> {
        let redis_conn =
            state
                .store
                .get_redis_conn()
                .change_context(errors::StorageError::RedisError(
                    errors::RedisError::RedisConnectionError.into(),
                ))?;

        let lock_key = Self::get_connector_customer_lock_key(connector_customer_id);

        // Get the id used to lock that key
        let stored_lock_value: String = redis_conn
            .get_key(&lock_key.clone().into())
            .await
            .map_err(|err| {
                tracing::error!(?err, "Failed to get lock key");
                errors::StorageError::RedisError(errors::RedisError::RedisConnectionError.into())
            })?;

        Some(stored_lock_value)
            .filter(|locked_value| locked_value == payment_id.get_string_repr())
            .ok_or_else(|| {
                tracing::warn!(
                    connector_customer_id = %connector_customer_id,
                    payment_id = %payment_id.get_string_repr(),
                    "Unlock attempt by non-lock owner",
                );
                errors::StorageError::RedisError(errors::RedisError::DeleteFailed.into())
            })?;

        match redis_conn.delete_key(&lock_key.into()).await {
            Ok(DelReply::KeyDeleted) => {
                tracing::debug!(
                    connector_customer_id = connector_customer_id,
                    "Connector customer unlocked"
                );
                Ok(true)
            }
            Ok(DelReply::KeyNotDeleted) => {
                tracing::debug!("Tried to unlock a stream which is already unlocked");
                Ok(false)
            }
            Err(err) => {
                tracing::error!(?err, "Failed to delete lock key");
                Ok(false)
            }
        }
    }

    /// Get all payment processor tokens for a connector customer
    #[instrument(skip_all)]
    pub async fn get_connector_customer_payment_processor_tokens(
        state: &SessionState,
        connector_customer_id: &str,
    ) -> CustomResult<HashMap<String, PaymentProcessorTokenStatus>, errors::StorageError> {
        let redis_conn =
            state
                .store
                .get_redis_conn()
                .change_context(errors::StorageError::RedisError(
                    errors::RedisError::RedisConnectionError.into(),
                ))?;
        let tokens_key = Self::get_connector_customer_tokens_key(connector_customer_id);

        let get_hash_err =
            errors::StorageError::RedisError(errors::RedisError::GetHashFieldFailed.into());

        let payment_processor_tokens: HashMap<String, String> = redis_conn
            .get_hash_fields(&tokens_key.into())
            .await
            .change_context(get_hash_err)?;

        let payment_processor_token_info_map: HashMap<String, PaymentProcessorTokenStatus> =
            payment_processor_tokens
                .into_iter()
                .filter_map(|(token_id, token_data)| {
                    match serde_json::from_str::<PaymentProcessorTokenStatus>(&token_data) {
                        Ok(token_status) => Some((token_id, token_status)),
                        Err(err) => {
                            tracing::warn!(
                                connector_customer_id = %connector_customer_id,
                                token_id = %token_id,
                                error = %err,
                                "Failed to deserialize token data, skipping",
                            );
                            None
                        }
                    }
                })
                .collect();
        tracing::debug!(
            connector_customer_id = connector_customer_id,
            "Fetched payment processor tokens",
        );

        Ok(payment_processor_token_info_map)
    }

    /// Find the most recent date from retry history
    pub fn find_nearest_date_from_current(
        retry_history: &HashMap<PrimitiveDateTime, i32>,
    ) -> Option<(PrimitiveDateTime, i32)> {
        let now_utc = OffsetDateTime::now_utc();
        let reference_time = PrimitiveDateTime::new(
            now_utc.date(),
            Time::from_hms(now_utc.hour(), 0, 0).unwrap_or(Time::MIDNIGHT),
        );

        retry_history
            .iter()
            .filter(|(date, _)| **date <= reference_time) // Only past dates + today
            .max_by_key(|(date, _)| *date) // Get the most recent
            .map(|(date, retry_count)| (*date, *retry_count))
    }

    /// Update connector customer payment processor tokens or add if doesn't exist
    #[instrument(skip_all)]
    pub async fn update_or_add_connector_customer_payment_processor_tokens(
        state: &SessionState,
        connector_customer_id: &str,
        payment_processor_token_info_map: HashMap<String, PaymentProcessorTokenStatus>,
    ) -> CustomResult<(), errors::StorageError> {
        let redis_conn =
            state
                .store
                .get_redis_conn()
                .change_context(errors::StorageError::RedisError(
                    errors::RedisError::RedisConnectionError.into(),
                ))?;
        let tokens_key = Self::get_connector_customer_tokens_key(connector_customer_id);

        // allocate capacity up-front to avoid rehashing
        let mut serialized_payment_processor_tokens: HashMap<String, String> =
            HashMap::with_capacity(payment_processor_token_info_map.len());

        // serialize all tokens, preserving explicit error handling and attachable diagnostics
        for (payment_processor_token_id, payment_processor_token_status) in
            payment_processor_token_info_map
        {
            let serialized = serde_json::to_string(&payment_processor_token_status)
                .change_context(errors::StorageError::SerializationFailed)
                .attach_printable("Failed to serialize token status")?;

            serialized_payment_processor_tokens.insert(payment_processor_token_id, serialized);
        }
        let seconds = &state.conf.revenue_recovery.redis_ttl_in_seconds;

        // Update or add tokens
        redis_conn
            .set_hash_fields(
                &tokens_key.into(),
                serialized_payment_processor_tokens,
                Some(*seconds),
            )
            .await
            .change_context(errors::StorageError::RedisError(
                errors::RedisError::SetHashFieldFailed.into(),
            ))?;

        tracing::info!(
            connector_customer_id = %connector_customer_id,
            "Successfully updated or added customer tokens",
        );

        Ok(())
    }

    /// Normalize retry window to exactly `RETRY_WINDOW_DAYS` days (today to `RETRY_WINDOW_DAYS - 1` days ago).
    pub fn normalize_retry_window(
        payment_processor_token: &mut PaymentProcessorTokenStatus,
        reference_time: PrimitiveDateTime,
    ) {
        let mut normalized_retry_history: HashMap<PrimitiveDateTime, i32> = HashMap::new();

        for hours_ago in 0..RETRY_WINDOW_IN_HOUR {
            let date = reference_time - Duration::hours(hours_ago.into());

            payment_processor_token
                .daily_retry_history
                .get(&date)
                .map(|&retry_count| {
                    normalized_retry_history.insert(date, retry_count);
                });
        }

        payment_processor_token.daily_retry_history = normalized_retry_history;
    }

    /// Get all payment processor tokens with retry information and wait times.
    pub fn get_tokens_with_retry_metadata(
        state: &SessionState,
        payment_processor_token_info_map: &HashMap<String, PaymentProcessorTokenStatus>,
    ) -> HashMap<String, PaymentProcessorTokenWithRetryInfo> {
        let today = OffsetDateTime::now_utc().date();
        let card_config = &state.conf.revenue_recovery.card_config;

        let mut result: HashMap<String, PaymentProcessorTokenWithRetryInfo> =
            HashMap::with_capacity(payment_processor_token_info_map.len());

        for (payment_processor_token_id, payment_processor_token_status) in
            payment_processor_token_info_map.iter()
        {
            let card_network = payment_processor_token_status
                .payment_processor_token_details
                .card_network
                .clone();

            // Calculate retry information.
            let retry_info = Self::payment_processor_token_retry_info(
                state,
                payment_processor_token_status,
                card_network.clone(),
            );

            // Determine the wait time (max of monthly and daily wait hours).
            let retry_wait_time_hours = retry_info
                .monthly_wait_hours
                .max(retry_info.daily_wait_hours);

            // Obtain network-specific limits and compute remaining monthly retries.
            let card_network_config = card_config.get_network_config(card_network);

            let monthly_retry_remaining = std::cmp::max(
                0,
                card_network_config.max_retry_count_for_thirty_day
                    - retry_info.total_30_day_retries,
            );

            // Build the per-token result struct.
            let token_with_retry_info = PaymentProcessorTokenWithRetryInfo {
                token_status: payment_processor_token_status.clone(),
                retry_wait_time_hours,
                monthly_retry_remaining,
                total_30_day_retries: retry_info.total_30_day_retries,
            };

            result.insert(payment_processor_token_id.clone(), token_with_retry_info);
        }
        tracing::debug!("Fetched payment processor tokens with retry metadata",);

        result
    }

    /// Sum retries over exactly the last 30 days
    fn calculate_total_30_day_retries(
        token: &PaymentProcessorTokenStatus,
        reference_time: PrimitiveDateTime,
    ) -> i32 {
        (0..RETRY_WINDOW_IN_HOUR)
            .map(|i| {
                let target_hour = reference_time - Duration::hours(i.into());

                token
                    .daily_retry_history
                    .get(&target_hour)
                    .copied()
                    .unwrap_or(INITIAL_RETRY_COUNT)
            })
            .sum()
    }

    /// Calculate wait hours
    fn calculate_wait_hours(target_date: PrimitiveDateTime, now: OffsetDateTime) -> i64 {
        let expiry_time = target_date.assume_utc();
        (expiry_time - now).whole_hours().max(0)
    }

    /// Calculate retry counts for exactly the last 30 days (hour-granular)
    pub fn payment_processor_token_retry_info(
        state: &SessionState,
        token: &PaymentProcessorTokenStatus,
        network_type: Option<CardNetwork>,
    ) -> TokenRetryInfo {
        let card_config = &state.conf.revenue_recovery.card_config;
        let card_network_config = card_config.get_network_config(network_type);

        let now_utc = OffsetDateTime::now_utc();
        let reference_time = PrimitiveDateTime::new(
            now_utc.date(),
            Time::from_hms(now_utc.hour(), 0, 0).unwrap_or(Time::MIDNIGHT),
        );

        // Total retries for last 720 hours
        let total_30_day_retries = Self::calculate_total_30_day_retries(token, reference_time);

        // Monthly wait-hour calculation ----
        let monthly_wait_hours =
            if total_30_day_retries >= card_network_config.max_retry_count_for_thirty_day {
                let mut accumulated_retries = 0;

                (0..RETRY_WINDOW_IN_HOUR)
                    .map(|i| reference_time - Duration::hours(i.into()))
                    .find(|window_hour| {
                        let retries = token
                            .daily_retry_history
                            .get(window_hour)
                            .copied()
                            .unwrap_or(0);
                        accumulated_retries += retries;

                        accumulated_retries >= card_network_config.max_retry_count_for_thirty_day
                    })
                    .map(|breach_hour| {
                        let allowed_at = breach_hour + Duration::days(31);
                        Self::calculate_wait_hours(allowed_at, now_utc)
                    })
                    .unwrap_or(0)
            } else {
                0
            };

        // Today's retries (using hourly buckets) ----
        let today_date = reference_time.date();

        let today_retries: i32 = (0..24)
            .map(|h| {
                let hour_bucket = PrimitiveDateTime::new(
                    today_date,
                    Time::from_hms(h, 0, 0).unwrap_or(Time::MIDNIGHT),
                );
                token
                    .daily_retry_history
                    .get(&hour_bucket)
                    .copied()
                    .unwrap_or(0)
            })
            .sum();

        let daily_wait_hours = if today_retries >= card_network_config.max_retries_per_day {
            let tomorrow_start =
                PrimitiveDateTime::new(today_date + Duration::days(1), Time::MIDNIGHT);
            Self::calculate_wait_hours(tomorrow_start, now_utc)
        } else {
            0
        };

        TokenRetryInfo {
            monthly_wait_hours,
            daily_wait_hours,
            total_30_day_retries,
        }
    }

    // Upsert payment processor token
    #[instrument(skip_all)]
    pub async fn upsert_payment_processor_token(
        state: &SessionState,
        connector_customer_id: &str,
        token_data: PaymentProcessorTokenStatus,
    ) -> CustomResult<bool, errors::StorageError> {
        let mut token_map =
            Self::get_connector_customer_payment_processor_tokens(state, connector_customer_id)
                .await?;

        let token_id = token_data
            .payment_processor_token_details
            .payment_processor_token
            .clone();
        let was_existing = token_map.contains_key(&token_id);

        let error_code = token_data.error_code.clone();

        let last_external_attempt_at = token_data.modified_at;

        let now_utc = OffsetDateTime::now_utc();
        let reference_time = PrimitiveDateTime::new(
            now_utc.date(),
            Time::from_hms(now_utc.hour(), 0, 0).unwrap_or(Time::MIDNIGHT),
        );

        token_map
            .get_mut(&token_id)
            .map(|existing_token| {
                Self::normalize_retry_window(existing_token, reference_time);

                for (date, &value) in &token_data.daily_retry_history {
                    existing_token
                        .daily_retry_history
                        .entry(*date)
                        .and_modify(|v| *v += value)
                        .or_insert(value);
                }
                existing_token.account_update_history = token_data.account_update_history.clone();
                existing_token.payment_processor_token_details =
                    token_data.payment_processor_token_details.clone();

                existing_token
                    .modified_at
                    .zip(last_external_attempt_at)
                    .and_then(|(existing_token_modified_at, last_external_attempt_at)| {
                        (last_external_attempt_at > existing_token_modified_at)
                            .then_some(last_external_attempt_at)
                    })
                    .or_else(|| {
                        existing_token
                            .modified_at
                            .is_none()
                            .then_some(last_external_attempt_at)
                            .flatten()
                    })
                    .map(|last_external_attempt_at| {
                        existing_token.modified_at = Some(last_external_attempt_at);
                        existing_token.error_code = error_code;
                        existing_token.is_hard_decline = token_data.is_hard_decline;
                        token_data
                            .is_active
                            .map(|is_active| existing_token.is_active = Some(is_active));
                    });
            })
            .or_else(|| {
                token_map.insert(token_id.clone(), token_data);
                None
            });

        Self::update_or_add_connector_customer_payment_processor_tokens(
            state,
            connector_customer_id,
            token_map,
        )
        .await?;
        tracing::debug!(
            connector_customer_id = connector_customer_id,
            "Upsert payment processor tokens",
        );

        Ok(!was_existing)
    }

    // Update payment processor token error code with billing connector response
    #[instrument(skip_all)]
    pub async fn update_payment_processor_token_error_code_from_process_tracker(
        state: &SessionState,
        connector_customer_id: &str,
        error_code: &Option<String>,
        is_hard_decline: &Option<bool>,
        payment_processor_token_id: Option<&str>,
    ) -> CustomResult<bool, errors::StorageError> {
        let now_utc = OffsetDateTime::now_utc();
        let reference_time = PrimitiveDateTime::new(
            now_utc.date(),
            Time::from_hms(now_utc.hour(), 0, 0).unwrap_or(Time::MIDNIGHT),
        );
        let updated_token = match payment_processor_token_id {
            Some(token_id) => {
                Self::get_connector_customer_payment_processor_tokens(state, connector_customer_id)
                    .await?
                    .values()
                    .find(|status| {
                        status
                            .payment_processor_token_details
                            .payment_processor_token
                            == token_id
                    })
                    .map(|status| PaymentProcessorTokenStatus {
                        payment_processor_token_details: status
                            .payment_processor_token_details
                            .clone(),
                        inserted_by_attempt_id: status.inserted_by_attempt_id.clone(),
                        error_code: error_code.clone(),
                        daily_retry_history: status.daily_retry_history.clone(),
                        scheduled_at: None,
                        is_hard_decline: *is_hard_decline,
                        modified_at: Some(PrimitiveDateTime::new(
                            OffsetDateTime::now_utc().date(),
                            OffsetDateTime::now_utc().time(),
                        )),
                        is_active: status.is_active,
                        account_update_history: status.account_update_history.clone(),
                        decision_threshold: status.decision_threshold,
                    })
            }
            None => None,
        };

        match updated_token {
            Some(mut token) => {
                Self::normalize_retry_window(&mut token, reference_time);

                match token.error_code {
                    None => token.daily_retry_history.clear(),
                    Some(_) => {
                        let current_count = token
                            .daily_retry_history
                            .get(&reference_time)
                            .copied()
                            .unwrap_or(INITIAL_RETRY_COUNT);
                        token
                            .daily_retry_history
                            .insert(reference_time, current_count + 1);
                    }
                }

                let mut tokens_map = HashMap::new();
                tokens_map.insert(
                    token
                        .payment_processor_token_details
                        .payment_processor_token
                        .clone(),
                    token.clone(),
                );

                Self::update_or_add_connector_customer_payment_processor_tokens(
                    state,
                    connector_customer_id,
                    tokens_map,
                )
                .await?;
                tracing::debug!(
                    connector_customer_id = connector_customer_id,
                    "Updated payment processor tokens with error code",
                );
                Ok(true)
            }
            None => {
                tracing::debug!(
                    connector_customer_id = connector_customer_id,
                    "No Token found with token id to update error code",
                );
                Ok(false)
            }
        }
    }

    // Update all payment processor token schedule time to None
    #[instrument(skip_all)]
    pub async fn update_payment_processor_tokens_schedule_time_to_none(
        state: &SessionState,
        connector_customer_id: &str,
    ) -> CustomResult<(), errors::StorageError> {
        let tokens_map =
            Self::get_connector_customer_payment_processor_tokens(state, connector_customer_id)
                .await?;

        let mut updated_tokens_map = HashMap::new();

        for (token_id, status) in tokens_map {
            let updated_status = PaymentProcessorTokenStatus {
                payment_processor_token_details: status.payment_processor_token_details.clone(),
                inserted_by_attempt_id: status.inserted_by_attempt_id.clone(),
                error_code: status.error_code.clone(),
                daily_retry_history: status.daily_retry_history.clone(),
                scheduled_at: None,
                is_hard_decline: status.is_hard_decline,
                modified_at: Some(PrimitiveDateTime::new(
                    OffsetDateTime::now_utc().date(),
                    OffsetDateTime::now_utc().time(),
                )),
                is_active: status.is_active,
                account_update_history: status.account_update_history.clone(),
                decision_threshold: status.decision_threshold,
            };
            updated_tokens_map.insert(token_id, updated_status);
        }

        Self::update_or_add_connector_customer_payment_processor_tokens(
            state,
            connector_customer_id,
            updated_tokens_map,
        )
        .await?;

        tracing::debug!(
            connector_customer_id = connector_customer_id,
            "Updated all payment processor tokens schedule time to None",
        );

        Ok(())
    }

    // Update payment processor token schedule time
    #[instrument(skip_all)]
    pub async fn update_payment_processor_token_schedule_time(
        state: &SessionState,
        connector_customer_id: &str,
        payment_processor_token: &str,
        schedule_time: Option<PrimitiveDateTime>,
        decision_threshold: Option<f64>,
    ) -> CustomResult<bool, errors::StorageError> {
        let updated_token =
            Self::get_connector_customer_payment_processor_tokens(state, connector_customer_id)
                .await?
                .values()
                .find(|status| {
                    status
                        .payment_processor_token_details
                        .payment_processor_token
                        == payment_processor_token
                })
                .map(|status| PaymentProcessorTokenStatus {
                    payment_processor_token_details: status.payment_processor_token_details.clone(),
                    inserted_by_attempt_id: status.inserted_by_attempt_id.clone(),
                    error_code: status.error_code.clone(),
                    daily_retry_history: status.daily_retry_history.clone(),
                    scheduled_at: schedule_time,
                    is_hard_decline: status.is_hard_decline,
                    modified_at: Some(PrimitiveDateTime::new(
                        OffsetDateTime::now_utc().date(),
                        OffsetDateTime::now_utc().time(),
                    )),
                    is_active: status.is_active,
                    account_update_history: status.account_update_history.clone(),
                    decision_threshold: decision_threshold.or(status.decision_threshold),
                });

        match updated_token {
            Some(token) => {
                let mut tokens_map = HashMap::new();
                tokens_map.insert(
                    token
                        .payment_processor_token_details
                        .payment_processor_token
                        .clone(),
                    token.clone(),
                );
                Self::update_or_add_connector_customer_payment_processor_tokens(
                    state,
                    connector_customer_id,
                    tokens_map,
                )
                .await?;
                tracing::debug!(
                    connector_customer_id = connector_customer_id,
                    "Updated payment processor tokens with schedule time",
                );
                Ok(true)
            }
            None => {
                tracing::debug!(
                    connector_customer_id = connector_customer_id,
                    "Payment processor tokens not found",
                );
                Ok(false)
            }
        }
    }

    // Get payment processor token with schedule time
    #[instrument(skip_all)]
    pub async fn get_payment_processor_token_with_schedule_time(
        state: &SessionState,
        connector_customer_id: &str,
    ) -> CustomResult<Option<PaymentProcessorTokenStatus>, errors::StorageError> {
        let tokens =
            Self::get_connector_customer_payment_processor_tokens(state, connector_customer_id)
                .await?;

        let scheduled_token = tokens
            .values()
            .find(|status| status.scheduled_at.is_some())
            .cloned();

        tracing::debug!(
            connector_customer_id = connector_customer_id,
            "Fetched payment processor token with schedule time",
        );

        Ok(scheduled_token)
    }

    // Get payment processor token using token id
    #[instrument(skip_all)]
    pub async fn get_payment_processor_token_using_token_id(
        state: &SessionState,
        connector_customer_id: &str,
        payment_processor_token: &str,
    ) -> CustomResult<Option<PaymentProcessorTokenStatus>, errors::StorageError> {
        // Get all tokens for the customer
        let tokens_map =
            Self::get_connector_customer_payment_processor_tokens(state, connector_customer_id)
                .await?;
        let token_details = tokens_map.get(payment_processor_token).cloned();

        tracing::debug!(
            token_found = token_details.is_some(),
            customer_id = connector_customer_id,
            "Fetched payment processor token & Checked existence ",
        );

        Ok(token_details)
    }

    // Check if all tokens are hard declined or no token found for the customer
    #[instrument(skip_all)]
    pub async fn are_all_tokens_hard_declined(
        state: &SessionState,
        connector_customer_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let tokens_map =
            Self::get_connector_customer_payment_processor_tokens(state, connector_customer_id)
                .await?;
        let all_hard_declined = tokens_map
            .values()
            .all(|token| token.is_hard_decline.unwrap_or(false));

        tracing::debug!(
            connector_customer_id = connector_customer_id,
            all_hard_declined,
            "Checked if all tokens are hard declined or no token found for the customer",
        );

        Ok(all_hard_declined)
    }

    // Get token based on retry type
    pub async fn get_token_based_on_retry_type(
        state: &SessionState,
        connector_customer_id: &str,
        retry_algorithm_type: RevenueRecoveryAlgorithmType,
        last_token_used: Option<&str>,
    ) -> CustomResult<Option<PaymentProcessorTokenStatus>, errors::StorageError> {
        let mut token = None;
        match retry_algorithm_type {
            RevenueRecoveryAlgorithmType::Monitoring => {
                logger::error!("Monitoring type found for Revenue Recovery retry payment");
            }

            RevenueRecoveryAlgorithmType::Cascading => {
                token = match last_token_used {
                    Some(token_id) => {
                        Self::get_payment_processor_token_using_token_id(
                            state,
                            connector_customer_id,
                            token_id,
                        )
                        .await?
                    }
                    None => None,
                };
            }

            RevenueRecoveryAlgorithmType::Smart => {
                token = Self::get_payment_processor_token_with_schedule_time(
                    state,
                    connector_customer_id,
                )
                .await?;
            }
        }

        let token = match token {
            Some(t) => {
                if t.is_hard_decline.unwrap_or(false) {
                    // Update the schedule time to None for hard declined tokens

                    logger::warn!(
                        connector_customer_id = connector_customer_id,
                        "Token is hard declined, setting schedule time to None"
                    );

                    Self::update_payment_processor_token_schedule_time(
                        state,
                        connector_customer_id,
                        &t.payment_processor_token_details.payment_processor_token,
                        None,
                        None,
                    )
                    .await?;

                    None
                } else {
                    Some(t)
                }
            }
            None => {
                logger::warn!(
                    connector_customer_id = connector_customer_id,
                    "No token found for the customer",
                );
                None
            }
        };

        Ok(token)
    }

    /// Get Redis key data for revenue recovery
    #[instrument(skip_all)]
    pub async fn get_redis_key_data_raw(
        state: &SessionState,
        connector_customer_id: &str,
        key_type: &RedisKeyType,
    ) -> CustomResult<(bool, i64, Option<serde_json::Value>), errors::StorageError> {
        let redis_conn =
            state
                .store
                .get_redis_conn()
                .change_context(errors::StorageError::RedisError(
                    errors::RedisError::RedisConnectionError.into(),
                ))?;

        let redis_key = match key_type {
            RedisKeyType::Status => Self::get_connector_customer_lock_key(connector_customer_id),
            RedisKeyType::Tokens => Self::get_connector_customer_tokens_key(connector_customer_id),
        };

        // Get TTL
        let ttl = redis_conn
            .get_ttl(&redis_key.clone().into())
            .await
            .map_err(|error| {
                tracing::error!(operation = "get_ttl", err = ?error);
                errors::StorageError::RedisError(errors::RedisError::GetHashFieldFailed.into())
            })?;

        // Get data based on key type and determine existence
        let (key_exists, data) = match key_type {
            RedisKeyType::Status => match redis_conn.get_key::<String>(&redis_key.into()).await {
                Ok(status_value) => (true, serde_json::Value::String(status_value)),
                Err(error) => {
                    tracing::error!(operation = "get_status_key", err = ?error);
                    (
                        false,
                        serde_json::Value::String(format!(
                            "Error retrieving status key: {}",
                            error
                        )),
                    )
                }
            },
            RedisKeyType::Tokens => {
                match redis_conn
                    .get_hash_fields::<HashMap<String, String>>(&redis_key.into())
                    .await
                {
                    Ok(hash_fields) => {
                        let exists = !hash_fields.is_empty();
                        let data = if exists {
                            serde_json::to_value(hash_fields).unwrap_or(serde_json::Value::Null)
                        } else {
                            serde_json::Value::Object(serde_json::Map::new())
                        };
                        (exists, data)
                    }
                    Err(error) => {
                        tracing::error!(operation = "get_tokens_hash", err = ?error);
                        (false, serde_json::Value::Null)
                    }
                }
            }
        };

        tracing::debug!(
            connector_customer_id = connector_customer_id,
            key_type = ?key_type,
            exists = key_exists,
            ttl = ttl,
            "Retrieved Redis key data"
        );

        Ok((key_exists, ttl, Some(data)))
    }

    /// Update Redis token with comprehensive card data
    #[instrument(skip_all)]
    pub async fn update_redis_token_with_comprehensive_card_data(
        state: &SessionState,
        customer_id: &str,
        token: &str,
        card_data: &revenue_recovery_data_backfill::ComprehensiveCardData,
        cutoff_datetime: Option<PrimitiveDateTime>,
    ) -> CustomResult<(), errors::StorageError> {
        // Get existing token data
        let mut token_map =
            Self::get_connector_customer_payment_processor_tokens(state, customer_id).await?;

        // Find the token to update
        let existing_token = token_map.get_mut(token).ok_or_else(|| {
            tracing::warn!(
                customer_id = customer_id,
                "Token not found in parsed Redis data - may be corrupted or missing for "
            );
            error_stack::Report::new(errors::StorageError::ValueNotFound(
                "Token not found in Redis".to_string(),
            ))
        })?;

        // Update the token details with new card data
        card_data.card_type.as_ref().map(|card_type| {
            existing_token.payment_processor_token_details.card_type = Some(card_type.clone())
        });

        card_data.card_exp_month.as_ref().map(|exp_month| {
            existing_token.payment_processor_token_details.expiry_month = Some(exp_month.clone())
        });

        card_data.card_exp_year.as_ref().map(|exp_year| {
            existing_token.payment_processor_token_details.expiry_year = Some(exp_year.clone())
        });

        card_data.card_network.as_ref().map(|card_network| {
            existing_token.payment_processor_token_details.card_network = Some(card_network.clone())
        });

        card_data.card_issuer.as_ref().map(|card_issuer| {
            existing_token.payment_processor_token_details.card_issuer = Some(card_issuer.clone())
        });

        // Update daily retry history if provided
        card_data
            .daily_retry_history
            .as_ref()
            .map(|retry_history| existing_token.daily_retry_history = retry_history.clone());

        // If cutoff_datetime is provided and existing scheduled_at < cutoff_datetime, set to None
        // If no scheduled_at value exists, leave it as None
        existing_token.scheduled_at = existing_token
            .scheduled_at
            .and_then(|existing_scheduled_at| {
                cutoff_datetime
                    .map(|cutoff| {
                        if existing_scheduled_at < cutoff {
                            tracing::info!(
                                customer_id = customer_id,
                                existing_scheduled_at = %existing_scheduled_at,
                                cutoff_datetime = %cutoff,
                                "Set scheduled_at to None because existing time is before cutoff time"
                            );
                            None
                        } else {
                            Some(existing_scheduled_at)
                        }
                    })
                    .unwrap_or(Some(existing_scheduled_at)) // No cutoff provided, keep existing value
            });

        existing_token.modified_at = Some(PrimitiveDateTime::new(
            OffsetDateTime::now_utc().date(),
            OffsetDateTime::now_utc().time(),
        ));

        // Update account_update_history if provided
        if let Some(history) = &card_data.account_update_history {
            // Convert api_models::AccountUpdateHistoryRecord to storage::AccountUpdateHistoryRecord
            let converted_history: Vec<AccountUpdateHistoryRecord> = history
                .iter()
                .map(|api_record| AccountUpdateHistoryRecord {
                    old_token: api_record.old_token.clone(),
                    new_token: api_record.new_token.clone(),
                    updated_at: api_record.updated_at,
                    old_token_info: api_record.old_token_info.clone(),
                    new_token_info: api_record.new_token_info.clone(),
                })
                .collect();
            existing_token
                .account_update_history
                .as_mut()
                .map(|data| data.extend(converted_history));
        }

        // Update is_active if provided
        card_data.is_active.map(|is_active| {
            existing_token.is_active = Some(is_active);
        });

        // Save the updated token map back to Redis
        Self::update_or_add_connector_customer_payment_processor_tokens(
            state,
            customer_id,
            token_map,
        )
        .await?;

        tracing::info!(
            customer_id = customer_id,
            "Updated Redis token data with comprehensive card data using struct"
        );

        Ok(())
    }
    pub async fn get_payment_processor_metadata_for_connector_customer(
        state: &SessionState,
        customer_id: &str,
    ) -> CustomResult<HashMap<String, PaymentProcessorTokenWithRetryInfo>, errors::StorageError>
    {
        let token_map =
            Self::get_connector_customer_payment_processor_tokens(state, customer_id).await?;

        let token_data = Self::get_tokens_with_retry_metadata(state, &token_map);

        Ok(token_data)
    }

    pub async fn handle_account_updater_token_update(
        state: &SessionState,
        customer_id: &str,
        scheduled_token: &PaymentProcessorTokenStatus,
        mandate_data: Option<api_models::payments::MandateIds>,
        payment_attempt_id: &id_type::GlobalAttemptId,
    ) -> CustomResult<AccountUpdaterAction, errors::StorageError> {
        match mandate_data {
            Some(data) => {
                logger::info!(
                    customer_id = customer_id,
                    "Mandate data provided, proceeding with token update."
                );

                let old_token_id = scheduled_token
                    .payment_processor_token_details
                    .payment_processor_token
                    .clone();

                let account_updater_action =
                    Self::determine_account_updater_action_based_on_old_token_and_mandate_data(
                        old_token_id.as_str(),
                        data,
                    )?;

                Ok(account_updater_action)
            }
            None => {
                logger::info!(
                    customer_id = customer_id,
                    "Skipping token update. Since we didn't get any updated mandate data"
                );
                Ok(AccountUpdaterAction::NoAction)
            }
        }
    }

    fn determine_account_updater_action_based_on_old_token_and_mandate_data(
        old_token: &str,
        mandate_data: api_models::payments::MandateIds,
    ) -> CustomResult<AccountUpdaterAction, errors::StorageError> {
        let new_token = mandate_data.get_connector_mandate_id();
        let account_updater_action = match new_token {
            Some(new_token) => {
                logger::info!("Found token in mandate data, comparing with old token");
                let is_token_equal = (new_token == old_token);

                logger::info!(
                    "Old token and new token comparison result: {}",
                    is_token_equal
                );

                if is_token_equal {
                    logger::info!("Old token and new token are equal. Checking for expiry update");
                    match mandate_data.get_updated_mandate_details_of_connector_mandate_id() {
                        Some(metadata) => {
                            logger::info!("Mandate metadata found for expiry update.");
                            AccountUpdaterAction::ExpiryUpdate(metadata)
                        }
                        None => {
                            logger::info!("No mandate metadata found for expiry update.");
                            AccountUpdaterAction::ExistingToken
                        }
                    }
                } else {
                    logger::info!("Old token and new token are not equal.");
                    match mandate_data.get_updated_mandate_details_of_connector_mandate_id() {
                        Some(metadata) => {
                            logger::info!("Mandate metadata found for token update.");
                            AccountUpdaterAction::TokenUpdate(new_token, metadata)
                        }
                        None => {
                            logger::warn!("No mandate metadata found for token update. No further action is taken");
                            AccountUpdaterAction::NoAction
                        }
                    }
                }
            }
            None => {
                logger::warn!("No new token found in mandate data while comparing with old token.");
                AccountUpdaterAction::NoAction
            }
        };

        Ok(account_updater_action)
    }
}

pub enum AccountUpdaterAction {
    TokenUpdate(String, api_models::payments::UpdatedMandateDetails),
    ExpiryUpdate(api_models::payments::UpdatedMandateDetails),
    ExistingToken,
    NoAction,
}

impl AccountUpdaterAction {
    pub async fn handle_account_updater_action(
        &self,
        state: &SessionState,
        customer_id: &str,
        scheduled_token: &PaymentProcessorTokenStatus,
        attempt_id: &id_type::GlobalAttemptId,
    ) -> CustomResult<(), errors::StorageError> {
        match self {
            Self::TokenUpdate(new_token, updated_mandate_details) => {
                logger::info!("Handling TokenUpdate action with new token");
                // Implement token update logic here using additional_card_info if needed

                let mut updated_token = scheduled_token.clone();
                updated_token.is_active = Some(false);
                updated_token.modified_at = Some(PrimitiveDateTime::new(
                    OffsetDateTime::now_utc().date(),
                    OffsetDateTime::now_utc().time(),
                ));

                RedisTokenManager::upsert_payment_processor_token(
                    state,
                    customer_id,
                    updated_token,
                )
                .await?;

                logger::info!("Successfully deactivated old token.");

                let new_token = PaymentProcessorTokenStatus {
                    payment_processor_token_details: PaymentProcessorTokenDetails {
                        payment_processor_token: new_token.to_owned(),
                        expiry_month: updated_mandate_details.card_exp_month.clone(),
                        expiry_year: updated_mandate_details.card_exp_year.clone(),
                        card_issuer: None,
                        last_four_digits: None,
                        card_type: None,
                        card_network: updated_mandate_details.card_network.clone(),
                        card_isin: updated_mandate_details.card_isin.clone(),
                    },
                    inserted_by_attempt_id: attempt_id.to_owned(),
                    error_code: None,
                    daily_retry_history: HashMap::new(),
                    scheduled_at: None,
                    is_hard_decline: Some(false),
                    modified_at: Some(PrimitiveDateTime::new(
                        OffsetDateTime::now_utc().date(),
                        OffsetDateTime::now_utc().time(),
                    )),
                    is_active: Some(true),
                    account_update_history: Some(vec![AccountUpdateHistoryRecord {
                        old_token: scheduled_token
                            .payment_processor_token_details
                            .payment_processor_token
                            .clone(),
                        new_token: new_token.to_owned(),
                        updated_at: PrimitiveDateTime::new(
                            OffsetDateTime::now_utc().date(),
                            OffsetDateTime::now_utc().time(),
                        ),
                        old_token_info: Some(api_models::payments::AdditionalCardInfo::from(
                            &scheduled_token.payment_processor_token_details,
                        )),
                        new_token_info: Some(api_models::payments::AdditionalCardInfo::from(
                            updated_mandate_details,
                        )),
                    }]),
                    decision_threshold: None,
                };

                RedisTokenManager::upsert_payment_processor_token(state, customer_id, new_token)
                    .await?;
                logger::info!("Successfully updated token with new token information.")
            }
            Self::ExpiryUpdate(updated_mandate_details) => {
                logger::info!("Handling ExpiryUpdate action");
                // Implement expiry update logic here using additional_card_info

                let mut updated_token = scheduled_token.clone();
                updated_token.payment_processor_token_details.expiry_month =
                    updated_mandate_details.card_exp_month.clone();
                updated_token.payment_processor_token_details.expiry_year =
                    updated_mandate_details.card_exp_year.clone();
                updated_token.payment_processor_token_details.card_network =
                    updated_mandate_details.card_network.clone();
                updated_token.payment_processor_token_details.card_isin =
                    updated_mandate_details.card_isin.clone();
                updated_token.modified_at = Some(PrimitiveDateTime::new(
                    OffsetDateTime::now_utc().date(),
                    OffsetDateTime::now_utc().time(),
                ));
                updated_token
                    .account_update_history
                    .get_or_insert_with(Vec::new)
                    .push(AccountUpdateHistoryRecord {
                        old_token: scheduled_token
                            .payment_processor_token_details
                            .payment_processor_token
                            .clone(),
                        new_token: updated_token
                            .payment_processor_token_details
                            .payment_processor_token
                            .clone(),
                        updated_at: PrimitiveDateTime::new(
                            OffsetDateTime::now_utc().date(),
                            OffsetDateTime::now_utc().time(),
                        ),
                        old_token_info: Some(api_models::payments::AdditionalCardInfo::from(
                            &scheduled_token.payment_processor_token_details,
                        )),
                        new_token_info: Some(api_models::payments::AdditionalCardInfo::from(
                            &updated_token.payment_processor_token_details,
                        )),
                    });

                RedisTokenManager::upsert_payment_processor_token(
                    state,
                    customer_id,
                    updated_token,
                )
                .await?;

                logger::info!("Successfully updated token expiry information.")
            }
            Self::ExistingToken => {
                logger::info!("Handling ExistingToken action - no changes needed");
                // No action needed for existing token
            }
            Self::NoAction => {
                logger::info!("No action to be taken for NoAction case");
                // No action needed
            }
        };

        Ok(())
    }
}
