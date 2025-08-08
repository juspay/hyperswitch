use std::collections::HashMap;

use common_enums::enums::CardNetwork;
use common_utils::{date_time, errors::CustomResult, id_type};
use error_stack::ResultExt;
use masking::Secret;
use redis_interface::{DelReply, SetnxReply};
use router_env::{instrument, tracing};
use serde::{Deserialize, Serialize};
use time::{Date, Duration, OffsetDateTime, PrimitiveDateTime};

use crate::{db::errors, types::storage::revenue_recovery::RetryLimitsConfig, SessionState};

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
    /// Lock connector customer status using SETNX
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

        let result: bool = match redis_conn
            .set_key_if_not_exists_with_expiry(&lock_key.into(), payment_id.get_string_repr(), None)
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
            Ok(DelReply::KeyDeleted) => Ok(true),
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

        let payment_processor_tokens: HashMap<String, String> = redis_conn
            .get_hash_fields(&tokens_key.into())
            .await
            .change_context(errors::StorageError::RedisError(
                errors::RedisError::GetHashFieldFailed.into(),
            ))?;

        let mut payment_processor_token_info_map = HashMap::new();

        for (token_id, payment_processor_token_data_str) in payment_processor_tokens {
            match serde_json::from_str::<PaymentProcessorTokenStatus>(
                &payment_processor_token_data_str,
            ) {
                Ok(token_status) => {
                    payment_processor_token_info_map.insert(token_id, token_status);
                }
                Err(error) => {
                    tracing::warn!(
                        connector_customer_id = %connector_customer_id,
                        token_id = %token_id,
                        error = %error,
                        "Failed to deserialize token data, skipping"
                    );
                }
            }
        }
        Ok(payment_processor_token_info_map)
    }

    /// Update connector customer payment processor tokens
    #[instrument(skip_all)]
    pub async fn update_connector_customer_payment_processor_tokens(
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

        // Serialize all tokens
        let mut serialized_payment_processor_tokens = HashMap::new();
        for (payment_processor_token_id, payment_processor_token_status) in
            payment_processor_token_info_map
        {
            let serialized = serde_json::to_string(&payment_processor_token_status)
                .change_context(errors::StorageError::SerializationFailed)
                .attach_printable("Failed to serialize token status")?;
            serialized_payment_processor_tokens.insert(payment_processor_token_id, serialized);
        }

        // Update all tokens in a single HSET operation
        redis_conn
            .set_hash_fields(
                &tokens_key.into(),
                serialized_payment_processor_tokens,
                None,
            )
            .await
            .change_context(errors::StorageError::RedisError(
                errors::RedisError::SetHashFieldFailed.into(),
            ))?;

        tracing::info!(
            connector_customer_id = %connector_customer_id,
            "Successfully updated customer tokens"
        );

        Ok(())
    }

    /// Get current date in yyyy-mm-dd format
    pub fn get_current_date() -> String {
        let today = date_time::now().date();
        format!(
            "{:04}-{:02}-{:02}",
            today.year(),
            today.month(),
            today.day()
        )
    }

    /// Normalize retry window to exactly 30 days (today to 29 days ago)
    pub fn normalize_retry_window(
        payment_processor_token: &mut PaymentProcessorTokenStatus,
        today: Date,
    ) {
        let mut normalized_map_for_retry_count = HashMap::new();

        // Create exactly 30 entries (today to 29 days ago)
        for i in 0..RETRY_WINDOW_DAYS {
            let date = today - Duration::days(i.into());
            let retry_count = payment_processor_token
                .daily_retry_history
                .get(&date)
                .copied()
                .unwrap_or(INITIAL_RETRY_COUNT);
            normalized_map_for_retry_count.insert(date, retry_count);
        }

        payment_processor_token.daily_retry_history = normalized_map_for_retry_count;
    }

    /// Get all payment processor tokens with retry information and wait times
    pub fn get_tokens_with_retry_metadata(
        state: &SessionState,
        payment_processor_token_info_map: &HashMap<String, PaymentProcessorTokenStatus>,
    ) -> HashMap<String, PaymentProcessorTokenWithRetryInfo> {
        let today = OffsetDateTime::now_utc().date();
        let mut result = HashMap::new();

        for (payment_processor_token_id, payment_processor_token_status) in
            payment_processor_token_info_map.iter()
        {
            // Calculate retry information
            let retry_info = Self::payment_processor_token_retry_info(
                state,
                payment_processor_token_status,
                today,
                payment_processor_token_status
                    .payment_processor_token_details
                    .card_network
                    .clone(),
            );

            // Calculate wait time
            let retry_wait_time_hours = retry_info
                .monthly_wait_hours
                .max(retry_info.daily_wait_hours);

            // Calculate remaining retries in 30-day window
            let card_network_config = RetryLimitsConfig::get_network_config(
                payment_processor_token_status
                    .payment_processor_token_details
                    .card_network
                    .clone(),
                state,
            );
            let monthly_retry_remaining = card_network_config
                .max_retries_last_30_days
                .saturating_sub(retry_info.total_30_day_retries);

            // Create the result struct with token info
            let token_with_retry_info = PaymentProcessorTokenWithRetryInfo {
                token_status: payment_processor_token_status.clone(),
                retry_wait_time_hours,
                monthly_retry_remaining,
            };

            result.insert(payment_processor_token_id.clone(), token_with_retry_info);
        }

        result
    }

    /// This function safely calculates retry counts for exactly the last 30 days
    pub fn payment_processor_token_retry_info(
        state: &SessionState,
        token: &PaymentProcessorTokenStatus,
        today: Date,
        network_type: Option<CardNetwork>,
    ) -> TokenRetryInfo {
        let card_network_config = RetryLimitsConfig::get_network_config(network_type, state);
        let now = OffsetDateTime::now_utc();

        //  Calculate total for exactly the last 30 days (rolling window)
        let mut total_30_day_retries: i32 = 0;
        for i in 0..RETRY_WINDOW_DAYS {
            let date = today - Duration::days(i.into());
            total_30_day_retries += token
                .daily_retry_history
                .get(&date)
                .copied()
                .unwrap_or(INITIAL_RETRY_COUNT);
        }

        // Check 30-day limit FIRST (monthly check)
        let monthly_limit_exceeded =
            total_30_day_retries >= card_network_config.max_retries_last_30_days;

        let monthly_wait_hours = if monthly_limit_exceeded {
            // Find the most recent oldest retry date within the 30-day window
            let oldest_retry_date = (0..RETRY_WINDOW_DAYS)
                .map(|i| today - Duration::days(i.into()))
                .find(|date| token.daily_retry_history.get(date).copied().unwrap_or(0) > 0);

            oldest_retry_date
                .map(|date| {
                    let expiry_time = (date + Duration::days(31)).midnight().assume_utc();
                    (expiry_time - now).whole_hours().max(0)
                })
                .unwrap_or(0) // No retry history found
        } else {
            0 // Monthly limit not exceeded
        };

        let today_retries = token
            .daily_retry_history
            .get(&today)
            .copied()
            .unwrap_or(INITIAL_RETRY_COUNT);

        let daily_wait_hours = if today_retries >= card_network_config.max_retries_per_day {
            let tomorrow = today + Duration::days(1);
            let tomorrow_midnight = tomorrow.midnight().assume_utc();
            (tomorrow_midnight - now).whole_hours().max(0)
        } else {
            0 // Daily limit not exceeded
        };

        TokenRetryInfo {
            monthly_wait_hours,
            daily_wait_hours,
            total_30_day_retries,
        }
    }

    /// Delete a specific payment processor token using token ID
    #[instrument(skip_all)]
    pub async fn delete_payment_processor_token_using_token_id(
        state: &SessionState,
        connector_customer_id: &str,
        payment_processor_token_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        // Get all existing payment processor tokens
        let mut payment_processor_token_info_map =
            Self::get_connector_customer_payment_processor_tokens(state, connector_customer_id)
                .await?;

        // Check if the token exists and remove it
        if payment_processor_token_info_map
            .remove(payment_processor_token_id)
            .is_none()
        {
            tracing::warn!(
                connector_customer_id = %connector_customer_id,
                "Token not found for deletion"
            );
            return Ok(false);
        }

        let redis_conn =
            state
                .store
                .get_redis_conn()
                .change_context(errors::StorageError::RedisError(
                    errors::RedisError::RedisConnectionError.into(),
                ))?;

        let tokens_key = format!("customer:{connector_customer_id}:tokens");

        // Delete entire Redis key
        redis_conn
            .delete_key(&tokens_key.into())
            .await
            .change_context(errors::StorageError::RedisError(
                errors::RedisError::DeleteFailed.into(),
            ))?;

        // Recreate hash with remaining tokens (if any)
        if !payment_processor_token_info_map.is_empty() {
            Self::update_connector_customer_payment_processor_tokens(
                state,
                connector_customer_id,
                payment_processor_token_info_map,
            )
            .await?;
        }

        tracing::info!("Successfully deleted payment processor token");

        Ok(true)
    }


    /// Upsert a payment processor token - insert if doesn't exist, update existing fields if it does
    #[instrument(skip_all)]
    pub async fn upsert_payment_processor_token(
        state: &SessionState,
        connector_customer_id: &str,
        token_data: PaymentProcessorTokenStatus,
    ) -> CustomResult<bool, errors::StorageError> {
        // Get existing tokens from Redis
        let mut payment_processor_token_info_map =
            Self::get_connector_customer_payment_processor_tokens(state, connector_customer_id)
                .await?;

        let payment_processor_token_id = token_data.payment_processor_token_details.payment_processor_token.clone();

        let was_existing = payment_processor_token_info_map.contains_key(&payment_processor_token_id);

        if was_existing {
            // Update existing token - merge the provided data with existing data
            if let Some(existing_token) = payment_processor_token_info_map.get_mut(&payment_processor_token_id) {
                // Update error code if provided
                if token_data.error_code.is_some() {
                    existing_token.error_code = token_data.error_code;
                }
                existing_token.scheduled_at = token_data.scheduled_at;
                
                // Merge daily retry history - keep existing history and add new entries
                let today = OffsetDateTime::now_utc().date();

                // Normalize retry window first (clean up old data and ensure 30-day window)
                Self::normalize_retry_window(existing_token, today);
                let current_retry_count = existing_token
                    .daily_retry_history
                    .get(&today)
                    .copied()
                    .unwrap_or(INITIAL_RETRY_COUNT);
                    existing_token
                    .daily_retry_history
                    .insert(today, current_retry_count + 1);
            }
        } else {
            // Insert new token
            payment_processor_token_info_map.insert(payment_processor_token_id.to_string(), token_data);
        }

        // Save the updated tokens back to Redis
        Self::update_connector_customer_payment_processor_tokens(
            state,
            connector_customer_id,
            payment_processor_token_info_map,
        )
        .await?;


        Ok(!was_existing) 
    }
    
}
