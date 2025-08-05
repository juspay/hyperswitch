use std::collections::HashMap;
use async_trait::async_trait;
use common_utils::{date_time, errors::CustomResult, id_type};
use time::{Date, OffsetDateTime};
use error_stack::ResultExt;
use router_env::{instrument, tracing};
use serde::{Deserialize, Serialize};
use storage_impl::{
    kv_router_store::KVRouterStore,
    redis::kv_store::RedisConnInterface,
    DatabaseStore,
};
use crate::SessionState;
use time::Duration;
use redis_interface::SetnxReply;
use redis_interface::DelReply;
use crate::types::storage::revenue_recovery::{RetryLimitsConfig, NetworkType};
use crate::db::errors;

// Constants for retry window management
const RETRY_WINDOW_DAYS: i64 = 30;
const INITIAL_RETRY_COUNT: u64 = 0;

// Redis key prefixes
const CUSTOMER_STATUS_KEY_PREFIX: &str = "customer";
const CUSTOMER_TOKENS_KEY_PREFIX: &str = "customer";

/// Represents the status and retry history of a payment processor token 
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentProcessorTokenStatus {
    /// Unique identifier for the token
    pub id: String,
    /// Payment network type (Visa, Mastercard, etc.)
    pub network: NetworkType,
    /// Payment intent ID that originally inserted this token
    pub inserted_by_payment_id: String,
    /// Error code associated with the token failure
    pub error_code: String,
    /// Daily retry count history for the last 30 days (date -> retry_count)
    pub daily_retry_history: HashMap<Date, u64>, 
}

/// Customer tokens data structure
#[derive(Debug, Clone)]
pub struct CustomerTokensData {
    pub connector_customer_id: id_type::CustomerId,
    pub payment_processor_token_info_list: HashMap<String, PaymentProcessorTokenStatus>,
}

/// Redis-based token management with customer locking
#[async_trait]
pub trait RedisTokenManager {
    // Core Redis operations
    async fn process_connector_customer_payment_processor_tokens(
        &self,
        connector_customer_id: &id_type::CustomerId,
        payment_id: &id_type::GlobalPaymentId,
        payment_intent_token_info: Vec<(String, String)>, // (token_id, error_code)
    ) -> CustomResult<Option<CustomerTokensData>, errors::StorageError>;

    /// Lock connector customer status 
    async fn lock_connector_customer_status(
        &self,
        connector_customer_id: &id_type::CustomerId,
        payment_id: &id_type::GlobalPaymentId,
    ) -> CustomResult<bool, errors::StorageError>;

    /// Unlock connector customer status
    async fn unlock_connector_customer_status(
        &self,
        connector_customer_id: &id_type::CustomerId,
    ) -> CustomResult<bool, errors::StorageError>;

    /// Get all payment processor tokens for a connector customer
    async fn get_connector_customer_payment_processor_tokens(
        &self,
        connector_customer_id: &id_type::CustomerId,
    ) -> CustomResult<HashMap<String, PaymentProcessorTokenStatus>, errors::StorageError>;

    /// Update connector customer payment processor tokens
    async fn update_connector_customer_payment_processor_tokens(
        &self,
        connector_customer_id: &id_type::CustomerId,
        payment_processor_token_info_list: HashMap<String, PaymentProcessorTokenStatus>,
    ) -> CustomResult<(), errors::StorageError>;

    /// Get current date in yyyy-mm-dd format
    fn get_current_date(&self) -> String;

    /// Normalize retry window to exactly 30 days (today to 29 days ago)
    fn normalize_retry_window(&self, token: &mut PaymentProcessorTokenStatus, today: Date);

    /// Select payment processor token with lowest retry count from valid tokens
    fn select_payment_processor_token_with_lowest_retries(&self, payment_processor_token_info_list: HashMap<String, PaymentProcessorTokenStatus>) -> Option<(String, PaymentProcessorTokenStatus)>;

    /// Check if payment processor token is within retry limits (both daily and 30-day)
    fn is_payment_processor_token_threshold_exceeded(
        &self,
        state: &SessionState,
        token: &PaymentProcessorTokenStatus,
        today: Date,
        network_type: NetworkType,
    ) -> bool;

    /// Filter out payment processor tokens that exceed daily or 30-day retry limits
    fn filter_payment_processor_tokens_by_retry_limits(
        &self,
        state: &SessionState,
        payment_processor_token_info_list: &HashMap<String, PaymentProcessorTokenStatus>,
    ) -> HashMap<String, PaymentProcessorTokenStatus>;

    /// Find the best payment processor token for a connector customer 
    async fn select_best_payment_processor_token_for_connector_customer(
        &self,
        state: &SessionState,
        connector_customer_id: &id_type::CustomerId,
        payment_id: &id_type::GlobalPaymentId,
        payment_intent_token_info: Vec<(String, String)>, // (token_id, error_code)
    ) -> CustomResult<Option<(String, OffsetDateTime)>, errors::StorageError>;

    /// Delete a specific payment processor token using token ID
    async fn delete_payment_processor_token_using_token_id(
        &self,
        connector_customer_id: &id_type::CustomerId,
        payment_processor_token_id: &str,
    ) -> CustomResult<bool, errors::StorageError>;

    /// Update error code for a specific payment processor token
    async fn update_payment_processor_token_error_code(
        &self,
        connector_customer_id: &id_type::CustomerId,
        payment_processor_token_id: &str,
        error_code: String,
    ) -> CustomResult<bool, errors::StorageError>;
}

#[async_trait]
impl<T: DatabaseStore + Send + Sync> RedisTokenManager for KVRouterStore<T> {
    #[instrument(skip_all)]
    async fn process_connector_customer_payment_processor_tokens(
        &self,
        connector_customer_id: &id_type::CustomerId,
        payment_id: &id_type::GlobalPaymentId,
        payment_intent_token_info: Vec<(String, String)>, 
    ) -> CustomResult<Option<CustomerTokensData>, errors::StorageError> {
        // Try to lock connector customer status
        if !self.lock_connector_customer_status(connector_customer_id, payment_id).await? {
            tracing::info!(
                connector_customer_id = connector_customer_id.get_string_repr(),
                payment_id = payment_id.get_string_repr(),
                "Customer is already locked by another invoice"
            );
            return Ok(None);
        }
    
        // Fetch all existing payment processor tokens 
        let mut existing_payment_processor_tokens = match self.get_connector_customer_payment_processor_tokens(connector_customer_id).await {
            Ok(payment_processor_token_info_list) => payment_processor_token_info_list,
            Err(redis_error) => {
                let _ = self.unlock_connector_customer_status(connector_customer_id).await;
                return Err(redis_error);
            }
        };
    
    
        for (payment_processor_token_id, error_code) in &payment_intent_token_info {
            if !existing_payment_processor_tokens.contains_key(payment_processor_token_id) {
                let new_payment_processor_token = PaymentProcessorTokenStatus {
                    id: payment_processor_token_id.clone(),
                    network: NetworkType::Visa, // Default network, should be provided in token_list
                    inserted_by_payment_id: payment_id.get_string_repr().to_string(),
                    error_code: error_code.clone(),
                    daily_retry_history: HashMap::new(),
                };
                existing_payment_processor_tokens.insert(payment_processor_token_id.clone(), new_payment_processor_token);
            }
        }

    
        Ok(Some(CustomerTokensData {
            connector_customer_id: connector_customer_id.clone(),
            payment_processor_token_info_list: existing_payment_processor_tokens,
        }))
    }

    #[instrument(skip_all)]
    async fn lock_connector_customer_status(
        &self,
        connector_customer_id: &id_type::CustomerId,
        payment_id: &id_type::GlobalPaymentId,
    ) -> CustomResult<bool, errors::StorageError> {
        let redis_conn = self
            .get_redis_conn()
            .change_context(errors::StorageError::KVError)?;

        let lock_key = format!("customer:{}:status", connector_customer_id.get_string_repr());
        
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
            connector_customer_id = connector_customer_id.get_string_repr(),
            payment_id = payment_id.get_string_repr(),
            lock_acquired = %result,
            "Connector customer lock attempt"
        );

        Ok(result)
    }

    #[instrument(skip_all)]
    async fn unlock_connector_customer_status(
        &self,
        connector_customer_id: &id_type::CustomerId,
    ) -> CustomResult<bool, errors::StorageError> {
        let redis_conn = self.get_redis_conn()
            .change_context(errors::StorageError::KVError)?;

        let lock_key = format!("customer:{}:status", connector_customer_id.get_string_repr());
        
        match redis_conn.delete_key(&lock_key.into()).await {
            Ok(DelReply::KeyDeleted) => Ok(true),
            Ok(DelReply::KeyNotDeleted) => {
                tracing::error!("Tried to unlock a stream which is already unlocked");
                Ok(false)
            }
            Err(err) => {
                tracing::error!(?err, "Failed to delete lock key");
                Ok(false)
            }
        }
    }

    #[instrument(skip_all)]
    async fn get_connector_customer_payment_processor_tokens(
        &self,
        connector_customer_id: &id_type::CustomerId,
    ) -> CustomResult<HashMap<String, PaymentProcessorTokenStatus>, errors::StorageError> {
        let redis_conn = self
            .get_redis_conn()
            .change_context(errors::StorageError::KVError)?;

        let tokens_key = format!("customer:{}:tokens", connector_customer_id.get_string_repr());
        
        let payment_processor_tokens: HashMap<String, String> = redis_conn
            .get_hash_fields(&tokens_key.into())
            .await
            .change_context(errors::StorageError::KVError)?;


        let mut payment_processor_token_info_list = HashMap::new();
        
        for (token_id, payment_processor_token_data_str) in payment_processor_tokens {
            match serde_json::from_str::<PaymentProcessorTokenStatus>(&payment_processor_token_data_str) {
                Ok(token_status) => {
                    payment_processor_token_info_list.insert(token_id, token_status);
                }
                Err(error) => {
                    tracing::warn!(
                        connector_customer_id = %connector_customer_id.get_string_repr(),
                        token_id = %token_id,
                        error = %error,
                        "Failed to deserialize token data, skipping"
                    );
                }
            }
        }

        tracing::debug!(
            connector_customer_id = %connector_customer_id.get_string_repr(),
            token_count = %payment_processor_token_info_list.len(),
            "Retrieved customer payment_processor_token_info_list"
        );

        Ok(payment_processor_token_info_list)
    }

    #[instrument(skip_all)]
    async fn update_connector_customer_payment_processor_tokens(
        &self,
        connector_customer_id: &id_type::CustomerId,
        payment_processor_token_info_list: HashMap<String, PaymentProcessorTokenStatus>,
    ) -> CustomResult<(), errors::StorageError> {
        let redis_conn = self
            .get_redis_conn()
            .change_context(errors::StorageError::KVError)?;

        let tokens_key = format!("customer:{}:tokens", connector_customer_id.get_string_repr());
        
        // Serialize all tokens
        let mut serialized_payment_processor_tokens = HashMap::new();
        for (payment_processor_token_id, payment_processor_token_status) in payment_processor_token_info_list {
            let serialized = serde_json::to_string(&payment_processor_token_status)
                .change_context(errors::StorageError::SerializationFailed)
                .attach_printable("Failed to serialize token status")?;
            serialized_payment_processor_tokens.insert(payment_processor_token_id, serialized);
        }

        // Update all tokens in a single HSET operation
        redis_conn
            .set_hash_fields(&tokens_key.into(), serialized_payment_processor_tokens,None)
            .await
            .change_context(errors::StorageError::KVError)?;

        tracing::info!(
            connector_customer_id = %connector_customer_id.get_string_repr(),
            "Successfully updated customer tokens"
        );

        Ok(())
    }

    fn get_current_date(&self) -> String {
        let today = date_time::now().date();
        format!("{:04}-{:02}-{:02}", today.year(), today.month() as u8, today.day())
    }

    fn normalize_retry_window(&self, payment_processor_token: &mut PaymentProcessorTokenStatus, today: Date) {
        let mut normalized_map_for_retry_count = HashMap::new();
        
        // Create exactly 30 entries (today to 29 days ago)
        for i in 0..RETRY_WINDOW_DAYS {
            let date = today - Duration::days(i);
            let retry_count = payment_processor_token.daily_retry_history.get(&date).copied().unwrap_or(INITIAL_RETRY_COUNT);
            normalized_map_for_retry_count.insert(date, retry_count);
        }
        
        payment_processor_token.daily_retry_history = normalized_map_for_retry_count;
    }

    fn select_payment_processor_token_with_lowest_retries(&self, payment_processor_token_info_list: HashMap<String, PaymentProcessorTokenStatus>) -> Option<(String, PaymentProcessorTokenStatus)> {
        let mut best_token = None;
        let mut lowest_retry_count = u64::MAX;
        
        for (token_id, token_status) in payment_processor_token_info_list {
            let total_retries: u64 = token_status.daily_retry_history.values().sum();
            
            if total_retries < lowest_retry_count {
                lowest_retry_count = total_retries;
                best_token = Some((token_id, token_status));
            }
        }
        
        best_token
    }

    
    fn is_payment_processor_token_threshold_exceeded(
        &self,
        state: &SessionState,
        token: &PaymentProcessorTokenStatus,
        today: Date,
        network_type: NetworkType,
    ) -> bool {
        let card_network_config = RetryLimitsConfig::get_network_config(network_type, state);

        // Check daily limit
        let today_retries = token.daily_retry_history.get(&today).copied().unwrap_or(INITIAL_RETRY_COUNT);
        if today_retries >= card_network_config.max_daily_retry_count {
            return false;
        }

        // Check 30-day limit
        let total_retries: u64 = token.daily_retry_history.values().sum();
        if total_retries >= card_network_config.retry_count_30_day {
            return false;
        }

        true
    }

    fn filter_payment_processor_tokens_by_retry_limits(
        &self,
        state: &SessionState,
        payment_processor_token_info_list: &HashMap<String, PaymentProcessorTokenStatus>,
    ) -> HashMap<String, PaymentProcessorTokenStatus> {
        let today = OffsetDateTime::now_utc().date();

        payment_processor_token_info_list
            .iter()
            .filter_map(|(payment_processor_token_id, payment_processor_token_status)| {
                let mut payment_processor_token_status = payment_processor_token_status.clone(); 
                self.normalize_retry_window(&mut payment_processor_token_status, today);

                if self.is_payment_processor_token_threshold_exceeded(state, &payment_processor_token_status, today, payment_processor_token_status.network.clone()) {
                    Some((payment_processor_token_id.clone(), payment_processor_token_status))
                } else {
                    tracing::info!(
                        payment_processor_token_id = %payment_processor_token_id,
                        card_network = ?payment_processor_token_status.network,
                        "Token filtered out due to retry limit exceeded"
                    );
                    None
                }
            })
            .collect()
    }

    #[instrument(skip_all)]
    async fn select_best_payment_processor_token_for_connector_customer(
        &self,
        state: &SessionState,
        connector_customer_id: &id_type::CustomerId,
        payment_id: &id_type::GlobalPaymentId,
        payment_intent_token_info: Vec<(String, String)>, // (token_id, error_code)
    ) -> CustomResult<Option<(String, OffsetDateTime)>, errors::StorageError> {
        // Process connector customer payment processor tokens with locking
        let mut customer_data = match self.process_connector_customer_payment_processor_tokens(connector_customer_id, payment_id, payment_intent_token_info).await? {
            Some(data) => data,
            None => return Ok(None), 
        };

        if customer_data.payment_processor_token_info_list.is_empty() {
            return Ok(None);
        }

        // Filter payment processor tokens by retry limits
        let available_payment_processor_tokens = self.filter_payment_processor_tokens_by_retry_limits(state, &customer_data.payment_processor_token_info_list);

        if available_payment_processor_tokens.is_empty() {
            tracing::warn!(
                connector_customer_id = %connector_customer_id.get_string_repr(),
                "All tokens exceed retry limits"
            );
            let _ = self.unlock_connector_customer_status(connector_customer_id).await;
            return Ok(None);
        }

        let mut selected_payment_processor_token_id: Option<String> = None;

        // Select best payment processor token with lowest retry count
        if let Some((best_token_id, status)) = self.select_payment_processor_token_with_lowest_retries(available_payment_processor_tokens) {
            if let Some(payment_processor_token_status) = customer_data.payment_processor_token_info_list.get_mut(&best_token_id) {
                *payment_processor_token_status = status.clone();
                selected_payment_processor_token_id = Some(status.id.clone());
            }
        }

        // Update Redis
        if let Err(error) = self.update_connector_customer_payment_processor_tokens(connector_customer_id, customer_data.payment_processor_token_info_list.clone()).await {
            let _ = self.unlock_connector_customer_status(connector_customer_id).await;
            return Err(error);
        }

        // Return selected payment processor token and schedule time
        if let Some(token_id) = selected_payment_processor_token_id {
            let now = OffsetDateTime::now_utc();
            return Ok(Some((token_id, now)));
        }

        Ok(None)
    }

    #[instrument(skip_all)]
    async fn delete_payment_processor_token_using_token_id(
        &self,
        connector_customer_id: &id_type::CustomerId,
        payment_processor_token_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        // Get all existing payment processor tokens
        let mut payment_processor_token_info_list = self.get_connector_customer_payment_processor_tokens(connector_customer_id).await?;
        
        // Check if the token exists and remove it
        if payment_processor_token_info_list.remove(payment_processor_token_id).is_none() {
            tracing::warn!(
                connector_customer_id = %connector_customer_id.get_string_repr(),
                payment_processor_token_id = %payment_processor_token_id,
                "Token not found for deletion"
            );
            return Ok(false);
        }

        let redis_conn = self
            .get_redis_conn()
            .change_context(errors::StorageError::KVError)?;

        let tokens_key = format!("customer:{}:tokens", connector_customer_id.get_string_repr());
        
        // Delete entire Redis key
        redis_conn.delete_key(&tokens_key.into()).await
            .change_context(errors::StorageError::KVError)?;

        // Recreate hash with remaining tokens (if any)
        if !payment_processor_token_info_list.is_empty() {
            self.update_connector_customer_payment_processor_tokens(connector_customer_id, payment_processor_token_info_list).await?;
        }

        tracing::info!(
            connector_customer_id = %connector_customer_id.get_string_repr(),
            payment_processor_token_id = %payment_processor_token_id,
            "Successfully deleted payment processor token"
        );

        Ok(true)
    }

    #[instrument(skip_all)]
    async fn update_payment_processor_token_error_code(
        &self,
        connector_customer_id: &id_type::CustomerId,
        payment_processor_token_id: &str,
        error_code: String,
    ) -> CustomResult<bool, errors::StorageError> {
        // Get all existing payment processor tokens
        let mut payment_processor_token_info_list = self.get_connector_customer_payment_processor_tokens(connector_customer_id).await?;
        
        // Find and update the specific token
        if let Some(payment_processor_token_status) = payment_processor_token_info_list.get_mut(payment_processor_token_id) {
            // Update only the error code, keeping all other fields unchanged
            payment_processor_token_status.error_code = error_code.clone();

            // Update Redis with the modified token
            self.update_connector_customer_payment_processor_tokens(connector_customer_id, payment_processor_token_info_list).await?;

            tracing::info!(
                connector_customer_id = %connector_customer_id.get_string_repr(),
                payment_processor_token_id = %payment_processor_token_id,
                new_error_code = %error_code,
                "Successfully updated payment processor token error code"
            );

            Ok(true)
        } else {
            tracing::warn!(
                connector_customer_id = %connector_customer_id.get_string_repr(),
                payment_processor_token_id = %payment_processor_token_id,
                "Token not found for error code update"
            );
            Ok(false)
        }
    }
}
