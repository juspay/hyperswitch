use std::collections::HashMap;

use async_trait::async_trait;
use common_utils::{date_time, errors::CustomResult, id_type::CustomerId};
use time::{Date, OffsetDateTime};
use error_stack::ResultExt;
use router_env::{instrument, tracing};
use serde::{Deserialize, Serialize};
use storage_impl::{
    kv_router_store::KVRouterStore,
    redis::kv_store::RedisConnInterface,
    DatabaseStore,
};
use time::Duration;
use redis_interface::SetnxReply;
use redis_interface::DelReply;

use crate::db::errors;

/// Network type for payment cards
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NetworkType {
    Visa,
    Mastercard,
    Amex,
    Discover,
}

/// Network-specific retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRetryConfig {
    pub max_daily_retry_count: u64,
    pub retry_count_30_day: u64,
}

/// Retry limits configuration for all networks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryLimitsConfig {
    pub mastercard: NetworkRetryConfig,
    pub visa: NetworkRetryConfig,
    pub amex: NetworkRetryConfig,
    pub discover: NetworkRetryConfig,
}

/// Token status structure with invoice tracking and retry history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStatus {
    pub id: String,
    pub network: NetworkType,
    pub locked_by_invoice_id: String,
    pub inserted_by_invoice_id: String,
    pub error_code: String,
    pub list_of_30_days: HashMap<Date, u64>, // date -> retry_count
}

/// Customer tokens data structure
#[derive(Debug, Clone)]
pub struct CustomerTokensData {
    pub customer_id: CustomerId,
    pub tokens: HashMap<String, TokenStatus>,
    pub locked_by_invoice_id: String,
}

/// Redis-based token management with customer locking
#[async_trait]
pub trait RedisTokenManager {
    /// Process customer tokens with locking mechanism
    /// 
    /// Workflow:
    /// 1. Lock customer_id_status if unlocked, else return None
    /// 2. Get entire customer_id HSET using customer_id
    /// 3. Check which tokens are missing and add them
    /// 4. Unlock customer_id_status
    /// 5. Return updated customer_id HSET
    async fn process_customer_tokens(
        &self,
        customer_id: CustomerId,
        invoice_id: &str,
        token_list: Vec<(String, String)>, // (token_id, error_code)
    ) -> CustomResult<Option<CustomerTokensData>, errors::StorageError>;

    /// Lock customer status using SETNX
    async fn lock_customer_status(
        &self,
        customer_id: CustomerId,
        invoice_id: &str,
    ) -> CustomResult<bool, errors::StorageError>;

    /// Unlock customer status
    async fn unlock_customer_status(
        &self,
        customer_id: CustomerId,
    ) -> CustomResult<bool, errors::StorageError>;

    /// Get all tokens for a customer
    async fn get_customer_tokens(
        &self,
        customer_id: CustomerId,
    ) -> CustomResult<HashMap<String, TokenStatus>, errors::StorageError>;

    /// Update customer tokens
    async fn update_customer_tokens(
        &self,
        customer_id: CustomerId,
        tokens: HashMap<String, TokenStatus>,
    ) -> CustomResult<(), errors::StorageError>;

    /// Add a new token for a customer
    // async fn add_token_for_customer(
    //     &self,
    //     customer_id: CustomerId,
    //     token_id: &str,
    //     invoice_id: &str,
    // ) -> CustomResult<(), errors::StorageError>;

    /// Update retry count for a token on current date
    async fn update_token_retry_count(
        &self,
        customer_id: CustomerId,
        token_id: &str,
        date: &str,
        retry_count: u64,
    ) -> CustomResult<(), errors::StorageError>;

    /// Get current date in yyyy-mm-dd format
    fn get_current_date(&self) -> String;
}

#[async_trait]
impl<T: DatabaseStore + Send + Sync> RedisTokenManager for KVRouterStore<T> {
    #[instrument(skip_all)]
    async fn process_customer_tokens(
        &self,
        customer_id: CustomerId,
        invoice_id: &str,
        token_list: Vec<(String, String)>, // (token_id, error_code)
    ) -> CustomResult<Option<CustomerTokensData>, errors::StorageError> {
        // Step 1: Try to lock customer status
        if !self.lock_customer_status(customer_id.clone(), invoice_id).await? {
            tracing::info!(
                customer_id = %customer_id.get_string_repr(),
                invoice_id = %invoice_id,
                "Customer is already locked by another invoice"
            );
            return Ok(None);
        }
    
        // Step 2: Fetch all existing tokens in one operation
        let mut all_tokens = match self.get_customer_tokens(customer_id.clone()).await {
            Ok(tokens) => tokens,
            Err(error) => {
                let _ = self.unlock_customer_status(customer_id.clone()).await;
                return Err(error);
            }
        };
    
        // Step 3: Merge missing tokens into existing tokens (in memory)
        let mut missing_tokens= HashMap::new();
        for (token_id, error_code) in &token_list {
            if !all_tokens.contains_key(token_id) {
                let new_token = TokenStatus {
                    id: token_id.clone(),
                    network: NetworkType::Visa, // Default network, should be provided in token_list
                    locked_by_invoice_id: String::new(),
                    inserted_by_invoice_id: invoice_id.to_string(),
                    error_code: error_code.clone(),
                    list_of_30_days: HashMap::new(),
                };
                missing_tokens.insert(token_id.clone(), new_token.clone());
                all_tokens.insert(token_id.clone(), new_token);
               
            }
        }
    
        // Step 4: Replace entire customer hash with combined data in single operation
        if !missing_tokens.is_empty() {
            if let Err(error) = self.update_customer_tokens(customer_id.clone(), all_tokens.clone()).await {
                let _ = self.unlock_customer_status(customer_id.clone()).await;
                return Err(error);
            }
        }
    
        // Step 5: Unlock customer status
        self.unlock_customer_status(customer_id.clone()).await?;
    
        tracing::info!(
            customer_id = %customer_id.get_string_repr(),
            invoice_id = %invoice_id,
            total_tokens = %all_tokens.len(),
            missing_tokens_added = %missing_tokens.len(),
            "Successfully processed customer tokens with single operation"
        );
    
        Ok(Some(CustomerTokensData {
            customer_id,
            tokens: all_tokens,
            locked_by_invoice_id: invoice_id.to_string(),
        }))
    }

    #[instrument(skip_all)]
    async fn lock_customer_status(
        &self,
        customer_id: CustomerId,
        invoice_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let redis_conn = self
            .get_redis_conn()
            .change_context(errors::StorageError::KVError)?;

        let lock_key = format!("customer:{}:status", customer_id.clone().get_string_repr());
        
        let result: bool = match redis_conn
        .set_key_if_not_exists_with_expiry(&lock_key.into(), invoice_id.to_string(), None)
        .await
        {
            Ok(resp) => resp == SetnxReply::KeySet,
            Err(error) => {
                tracing::error!(operation = "lock_stream", err = ?error);
                false
            }
        };

        tracing::debug!(
            customer_id = %customer_id.get_string_repr(),
            invoice_id = %invoice_id,
            lock_acquired = %result,
            "Customer lock attempt"
        );

        Ok(result)
    }

    #[instrument(skip_all)]
    async fn unlock_customer_status(
        &self,
        customer_id: CustomerId,
    ) -> CustomResult<bool, errors::StorageError> {
        let redis_conn = self.get_redis_conn()
            .change_context(errors::StorageError::KVError)?;

        let lock_key = format!("customer:{}:status", customer_id.get_string_repr());
        
        match redis_conn.delete_key(&lock_key.into()).await {
            Ok(DelReply::KeyDeleted) => Ok(true),
            Ok(DelReply::KeyNotDeleted) => {
                tracing::error!("Tried to unlock a stream which is already unlocked");
                Ok(false)
            }
            Err(err) => {
                tracing::error!(?err, "Failed to delete lock key");
                false
            }
        }
    }

    #[instrument(skip_all)]
    async fn get_customer_tokens(
        &self,
        customer_id: CustomerId,
    ) -> CustomResult<HashMap<String, TokenStatus>, errors::StorageError> {
        let redis_conn = self
            .get_redis_conn()
            .change_context(errors::StorageError::KVError)?;

        let tokens_key = format!("customer:{}:tokens", customer_id.get_string_repr());
        
        let tokens_data: HashMap<String, String> = redis_conn
            .get_hash_fields(&tokens_key.into())
            .await
            .change_context(errors::StorageError::KVError)?;


        let mut tokens = HashMap::new();
        
        for (token_id, token_data_str) in tokens_data {
            match serde_json::from_str::<TokenStatus>(&token_data_str) {
                Ok(token_status) => {
                    tokens.insert(token_id, token_status);
                }
                Err(error) => {
                    tracing::warn!(
                        customer_id = %customer_id.get_string_repr(),
                        token_id = %token_id,
                        error = %error,
                        "Failed to deserialize token data, skipping"
                    );
                }
            }
        }

        tracing::debug!(
            customer_id = %customer_id.get_string_repr(),
            token_count = %tokens.len(),
            "Retrieved customer tokens"
        );

        Ok(tokens)
    }

    #[instrument(skip_all)]
    async fn update_customer_tokens(
        &self,
        customer_id: CustomerId,
        tokens: HashMap<String, TokenStatus>,
    ) -> CustomResult<(), errors::StorageError> {
        let redis_conn = self
            .get_redis_conn()
            .change_context(errors::StorageError::KVError)?;

        let tokens_key = format!("customer:{}:tokens", customer_id.get_string_repr());
        
        // Serialize all tokens
        let mut serialized_tokens = HashMap::new();
        for (token_id, token_status) in tokens {
            let serialized = serde_json::to_string(&token_status)
                .change_context(errors::StorageError::SerializationFailed)
                .attach_printable("Failed to serialize token status")?;
            serialized_tokens.insert(token_id, serialized);
        }

        // Update all tokens in a single HSET operation
        redis_conn
            .set_hash_fields(&tokens_key.into(), serialized_tokens,None)
            .await
            .change_context(errors::StorageError::KVError)?;

        tracing::info!(
            customer_id = %customer_id.get_string_repr(),
            "Successfully updated customer tokens"
        );

        Ok(())
    }

    // #[instrument(skip_all)]
    // async fn add_token_for_customer(
    //     &self,
    //     customer_id: CustomerId,
    //     token_id: &str,
    //     invoice_id: &str,
    // ) -> CustomResult<(), errors::StorageError> {
    //     let redis_conn = self
    //         .get_redis_conn()
    //         .change_context(errors::StorageError::KVError)?;

    //     let tokens_key = format!("customer:{}:tokens", customer_id.get_string_repr());
        
    //     let token_status = TokenStatus {
    //         id: token_id.to_string(),
    //         locked_by_invoice_id: String::new(),
    //         inserted_by_invoice_id: invoice_id.to_string(),
    //         error_code: String::new(),
    //         list_of_31_days: HashMap::new(),
    //     };

    //     let serialized_token = serde_json::to_string(&token_status)
    //         .change_context(errors::StorageError::SerializationFailed)
    //         .attach_printable("Failed to serialize token status")?;

    //     let mut field_map = HashMap::new();
    //     field_map.insert(token_id.to_string(), serialized_token);
        
    //     redis_conn
    //         .set_hash_fields(&tokens_key.into(), field_map,None)
    //         .await
    //         .change_context(errors::StorageError::KVError)?;

    //     tracing::info!(
    //         customer_id = %customer_id.get_string_repr(),
    //         token_id = %token_id,
    //         invoice_id = %invoice_id,
    //         "Successfully added token for customer"
    //     );

    //     Ok(())
    // }

    #[instrument(skip_all)]
    async fn update_token_retry_count(
        &self,
        customer_id: CustomerId,
        token_id: &str,
        date: &str,
        increment_by: u64,
    ) -> CustomResult<(), errors::StorageError> {
        // Get current tokens
        let mut tokens = self.get_customer_tokens(customer_id.clone()).await?;
        
        // Update the specific token's retry count for the date
        if let Some(token_status) = tokens.get_mut(token_id) {
            let date_parsed = Date::parse(date, &time::format_description::parse("[year]-[month]-[day]").unwrap())
                .change_context(errors::StorageError::SerializationFailed)?;
            
            // Increment the retry count for the specified date
            let current_count = token_status.list_of_30_days.get(&date_parsed).copied().unwrap_or(0);
            token_status.list_of_30_days.insert(date_parsed, current_count + increment_by);
            
            // Clean up old entries (keep only last 30 days)
            let today = OffsetDateTime::now_utc().date();
            token_status.list_of_30_days.retain(|&date_key, _| {
                let days_diff = (today - date_key).whole_days();
                days_diff >= 0 && days_diff < 30
            });
            
            // Update the tokens
            self.update_customer_tokens(customer_id.clone(), tokens).await?;
            
            tracing::info!(
                customer_id = %customer_id.get_string_repr(),
                token_id = %token_id,
                date = %date,
                increment_by = %increment_by,
                new_count = %(current_count + increment_by),
                "Successfully updated token retry count"
            );
        } else {
            tracing::warn!(
                customer_id = %customer_id.get_string_repr(),
                token_id = %token_id,
                "Token not found for retry count update"
            );
        }

        Ok(())
    }

    fn get_current_date(&self) -> String {
        let today = date_time::now().date();
        format!("{:04}-{:02}-{:02}", today.year(), today.month() as u8, today.day())
    }
}

/// Normalize retry window to exactly 30 days (today to 29 days ago)
fn normalize_retry_window(token: &mut TokenStatus, today: Date) {
    let mut normalized_map = HashMap::new();
    
    // Create exactly 30 entries (today to 29 days ago)
    for i in 0..30 {
        let date = today - Duration::days(i);
        let retry_count = token.list_of_30_days.get(&date).copied().unwrap_or(0);
        normalized_map.insert(date, retry_count);
    }
    
    token.list_of_30_days = normalized_map;
}

/// Get network configuration for a specific network type
fn get_network_config(
    network: NetworkType,
    retry_limits_config: &RetryLimitsConfig,
) -> &NetworkRetryConfig {
    match network {
        NetworkType::Mastercard => &retry_limits_config.mastercard,
        NetworkType::Visa => &retry_limits_config.visa,
        NetworkType::Amex => &retry_limits_config.amex,
        NetworkType::Discover => &retry_limits_config.discover,
    }
}

/// Check if token is within retry limits (both daily and 30-day)
fn is_token_within_limits(
    token: &TokenStatus,
    retry_limits_config: &RetryLimitsConfig,
    today: Date,
) -> bool {
    let network_config = get_network_config(token.network.clone(), retry_limits_config);
    
    // Check daily limit
    let today_retries = token.list_of_30_days.get(&today).copied().unwrap_or(0);
    if today_retries >= network_config.max_daily_retry_count {
        return false;
    }
    
    // Check 30-day limit
    let total_retries: u64 = token.list_of_30_days.values().sum();
    if total_retries >= network_config.retry_count_30_day {
        return false;
    }
    
    true
}

/// Filter out tokens that exceed daily or 30-day retry limits
pub fn filter_tokens_by_retry_limits(
    tokens: HashMap<String, TokenStatus>,
    retry_limits_config: &RetryLimitsConfig,
) -> HashMap<String, TokenStatus> {
    let today = OffsetDateTime::now_utc().date();
    
    tokens
        .into_iter()
        .filter_map(|(token_id, mut token_status)| {
            // Normalize the retry window first
            normalize_retry_window(&mut token_status, today);
            
            // Check if token is within limits
            if is_token_within_limits(&token_status, retry_limits_config, today) {
                Some((token_id, token_status))
            } else {
                tracing::info!(
                    token_id = %token_id,
                    network = ?token_status.network,
                    "Token filtered out due to retry limit exceeded"
                );
                None
            }
        })
        .collect()
}

/// Select token with lowest retry count from valid tokens
fn select_token_with_lowest_retries(
    tokens: HashMap<String, TokenStatus>
) -> Option<(String, TokenStatus)> {
    let mut best_token = None;
    let mut lowest_retry_count = u64::MAX;
    
    for (token_id, token_status) in tokens {
        let total_retries: u64 = token_status.list_of_30_days.values().sum();
        
        if total_retries < lowest_retry_count {
            lowest_retry_count = total_retries;
            best_token = Some((token_id, token_status));
        }
    }
    
    best_token
}

/// Find the best token for a customer based on retry history with retry limits filtering
pub async fn find_best_token_for_customer<T: RedisTokenManager>(
    store: &T,
    customer_id: CustomerId,
    invoice_id: &str,
    token_list: Vec<(String, String)>, // (token_id, error_code)
    retry_limits_config: &RetryLimitsConfig,
) -> CustomResult<Option<(String, TokenStatus)>, errors::StorageError> {
    // Process customer tokens with locking
    let customer_data = match store.process_customer_tokens(customer_id.clone(), invoice_id, token_list).await? {
        Some(data) => data,
        None => return Ok(None), // Customer is locked by another invoice
    };

    if customer_data.tokens.is_empty() {
        return Ok(None);
    }

    // Filter tokens by retry limits
    let valid_tokens = filter_tokens_by_retry_limits(customer_data.tokens, retry_limits_config);
    
    if valid_tokens.is_empty() {
        tracing::warn!(
            customer_id = %customer_id.get_string_repr(),
            "All tokens exceed retry limits"
        );
        return Ok(None);
    }

    // Select best token with lowest retry count
    let best_token = select_token_with_lowest_retries(valid_tokens);

    // Update retry count for selected token
    if let Some((ref token_id, _)) = best_token {
        let today = store.get_current_date();
        store.update_token_retry_count(customer_id.clone(), token_id, &today, 1).await?;
        
        tracing::info!(
            customer_id = %customer_id.get_string_repr(),
            token_id = %token_id,
            date = %today,
            "Incremented retry count for selected token"
        );
    }

    Ok(best_token)
}

/// Get tokens with high retry counts for monitoring
pub async fn get_high_retry_tokens<T: RedisTokenManager>(
    store: &T,
    customer_id: CustomerId,
    invoice_id: &str,
    token_list: Vec<(String, String)>, // (token_id, error_code)
    retry_threshold: u64,
) -> CustomResult<HashMap<String, TokenStatus>, errors::StorageError> {
    let customer_data = match store.process_customer_tokens(customer_id, invoice_id, token_list).await? {
        Some(data) => data,
        None => return Ok(HashMap::new()), // Customer is locked
    };

    let high_retry_tokens = customer_data
        .tokens
        .into_iter()
        .filter(|(_, token_status)| {
            let total_retries: u64 = token_status.list_of_30_days.values().sum();
            total_retries >= retry_threshold
        })
        .collect();

    Ok(high_retry_tokens)
}
