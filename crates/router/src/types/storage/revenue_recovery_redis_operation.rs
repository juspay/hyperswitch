use std::collections::HashMap;

use common_enums::enums::CardNetwork;
use common_utils::{date_time, errors::CustomResult, id_type};
use error_stack::ResultExt;
use masking::Secret;
use redis_interface::{DelReply, SetnxReply};
use router_env::{instrument, logger, tracing};
use serde::{Deserialize, Serialize};
use time::{Date, Duration, OffsetDateTime, PrimitiveDateTime};

use crate::{db::errors, types::storage::enums::RevenueRecoveryAlgorithmType, SessionState};

// Constants for retry window management
const RETRY_WINDOW_DAYS: i32 = 30;
const INITIAL_RETRY_COUNT: i32 = 0;

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
    pub daily_retry_history: HashMap<Date, i32>,
    /// Scheduled time for the next retry attempt
    pub scheduled_at: Option<PrimitiveDateTime>,
    /// Indicates if the token is a hard decline (no retries allowed)
    pub is_hard_decline: Option<bool>,
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
}

/// Redis-based token management struct
pub struct RedisTokenManager;

impl RedisTokenManager {
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

        let lock_key = format!("customer:{connector_customer_id}:status");
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

    /// Unlock connector customer status
    #[instrument(skip_all)]
    pub async fn unlock_connector_customer_status(
        state: &SessionState,
        connector_customer_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let redis_conn =
            state
                .store
                .get_redis_conn()
                .change_context(errors::StorageError::RedisError(
                    errors::RedisError::RedisConnectionError.into(),
                ))?;

        let lock_key = format!("customer:{connector_customer_id}:status");

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
        let tokens_key = format!("customer:{connector_customer_id}:tokens");

        let get_hash_err =
            errors::StorageError::RedisError(errors::RedisError::GetHashFieldFailed.into());

        let payment_processor_tokens: HashMap<String, String> = redis_conn
            .get_hash_fields(&tokens_key.into())
            .await
            .change_context(get_hash_err)?;

        // build the result map using iterator adapters (explicit match preserved for logging)
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
        let tokens_key = format!("customer:{connector_customer_id}:tokens");

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

    /// Get current date in `yyyy-mm-dd` format.
    pub fn get_current_date() -> String {
        let today = date_time::now().date();

        let (year, month, day) = (today.year(), today.month(), today.day());

        format!("{year:04}-{month:02}-{day:02}",)
    }

    /// Normalize retry window to exactly `RETRY_WINDOW_DAYS` days (today to `RETRY_WINDOW_DAYS - 1` days ago).
    pub fn normalize_retry_window(
        payment_processor_token: &mut PaymentProcessorTokenStatus,
        today: Date,
    ) {
        let mut normalized_retry_history: HashMap<Date, i32> = HashMap::new();

        for days_ago in 0..RETRY_WINDOW_DAYS {
            let date = today - Duration::days(days_ago.into());

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
                today,
                card_network.clone(),
            );

            // Determine the wait time (max of monthly and daily wait hours).
            let retry_wait_time_hours = retry_info
                .monthly_wait_hours
                .max(retry_info.daily_wait_hours);

            // Obtain network-specific limits and compute remaining monthly retries.
            let card_network_config = card_config.get_network_config(card_network);

            let monthly_retry_remaining = card_network_config
                .max_retry_count_for_thirty_day
                .saturating_sub(retry_info.total_30_day_retries);

            // Build the per-token result struct.
            let token_with_retry_info = PaymentProcessorTokenWithRetryInfo {
                token_status: payment_processor_token_status.clone(),
                retry_wait_time_hours,
                monthly_retry_remaining,
            };

            result.insert(payment_processor_token_id.clone(), token_with_retry_info);
        }
        tracing::debug!("Fetched payment processor tokens with retry metadata",);

        result
    }

    /// Sum retries over exactly the last 30 days
    fn calculate_total_30_day_retries(token: &PaymentProcessorTokenStatus, today: Date) -> i32 {
        (0..RETRY_WINDOW_DAYS)
            .map(|i| {
                let date = today - Duration::days(i.into());
                token
                    .daily_retry_history
                    .get(&date)
                    .copied()
                    .unwrap_or(INITIAL_RETRY_COUNT)
            })
            .sum()
    }

    /// Calculate wait hours
    fn calculate_wait_hours(target_date: Date, now: OffsetDateTime) -> i64 {
        let expiry_time = target_date.midnight().assume_utc();
        (expiry_time - now).whole_hours().max(0)
    }

    /// Calculate retry counts for exactly the last 30 days
    pub fn payment_processor_token_retry_info(
        state: &SessionState,
        token: &PaymentProcessorTokenStatus,
        today: Date,
        network_type: Option<CardNetwork>,
    ) -> TokenRetryInfo {
        let card_config = &state.conf.revenue_recovery.card_config;
        let card_network_config = card_config.get_network_config(network_type);

        let now = OffsetDateTime::now_utc();

        let total_30_day_retries = Self::calculate_total_30_day_retries(token, today);

        let monthly_wait_hours =
            if total_30_day_retries >= card_network_config.max_retry_count_for_thirty_day {
                (0..RETRY_WINDOW_DAYS)
                    .map(|i| today - Duration::days(i.into()))
                    .find(|date| token.daily_retry_history.get(date).copied().unwrap_or(0) > 0)
                    .map(|date| Self::calculate_wait_hours(date + Duration::days(31), now))
                    .unwrap_or(0)
            } else {
                0
            };

        let today_retries = token
            .daily_retry_history
            .get(&today)
            .copied()
            .unwrap_or(INITIAL_RETRY_COUNT);

        let daily_wait_hours = if today_retries >= card_network_config.max_retries_per_day {
            Self::calculate_wait_hours(today + Duration::days(1), now)
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
        let today = OffsetDateTime::now_utc().date();

        token_map
            .get_mut(&token_id)
            .map(|existing_token| {
                error_code.map(|err| existing_token.error_code = Some(err));

                Self::normalize_retry_window(existing_token, today);

                for (date, &value) in &token_data.daily_retry_history {
                    existing_token
                        .daily_retry_history
                        .entry(*date)
                        .and_modify(|v| *v += value)
                        .or_insert(value);
                }
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
        let today = OffsetDateTime::now_utc().date();
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
                    })
            }
            None => None,
        };

        match updated_token {
            Some(mut token) => {
                Self::normalize_retry_window(&mut token, today);

                match token.error_code {
                    None => token.daily_retry_history.clear(),
                    Some(_) => {
                        let current_count = token
                            .daily_retry_history
                            .get(&today)
                            .copied()
                            .unwrap_or(INITIAL_RETRY_COUNT);
                        token.daily_retry_history.insert(today, current_count + 1);
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

    // Update payment processor token schedule time
    #[instrument(skip_all)]
    pub async fn update_payment_processor_token_schedule_time(
        state: &SessionState,
        connector_customer_id: &str,
        payment_processor_token: &str,
        schedule_time: Option<PrimitiveDateTime>,
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
                    "payment processor tokens with not found",
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

        token = token.and_then(|t| {
            t.is_hard_decline
                .unwrap_or(false)
                .then(|| {
                    logger::error!("Token is hard declined");
                })
                .map_or(Some(t), |_| None)
        });

        Ok(token)
    }
}
