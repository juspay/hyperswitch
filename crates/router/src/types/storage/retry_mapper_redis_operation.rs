use async_trait::async_trait;
use common_utils::{date_time, errors::CustomResult, id_type::CustomerId};
use error_stack::ResultExt;
use time::Duration;
use router_env::{instrument, tracing};
use storage_impl::{
    kv_router_store::KVRouterStore,
    redis::kv_store::RedisConnInterface,
    DatabaseStore,
};

use crate::db::errors;

/// Represents a rolling retry count over a specified time window
#[derive(Debug, Clone)]
pub struct RollingRetryCount {
    pub token_id: String,
    pub customer_id: CustomerId,
    pub total_count: u64,
    pub window_days: u8,
}

/// Request structure for batch retry count operations
#[derive(Debug, Clone)]
pub struct BatchRetryRequest {
    pub token_id: String,
    pub customer_id: CustomerId,
}

// Lua script for atomic increment with expiration
// This ensures consistency and reduces network round trips
const INCREMENT_RETRY_COUNT_SCRIPT: &str = r#"
local key = 'retry:' .. ARGV[1] .. ':' .. ARGV[2] .. ':' .. ARGV[3]
local count = redis.call('INCRBY', key, 1)
redis.call('EXPIRE', key, 2678400) -- 31 days in seconds (31 * 24 * 60 * 60)
return count
"#;

// Lua script for efficient rolling window calculation
// Takes pre-calculated keys and sums their values efficiently on the server side
const GET_ROLLING_RETRY_COUNT_SCRIPT: &str = r#"
local total = 0
for i = 1, #KEYS do
    local value = redis.call('GET', KEYS[i])
    if value then
        total = total + tonumber(value)
    end
end
return total
"#;

// Lua script for batch processing multiple token-customer pairs
// Processes multiple pairs efficiently in a single Redis operation
const BATCH_ROLLING_RETRY_COUNT_SCRIPT: &str = r#"
local results = {}
local keys_per_pair = tonumber(ARGV[1])
local num_pairs = #KEYS / keys_per_pair

for i = 1, num_pairs do
    local pair_total = 0
    local start_idx = (i - 1) * keys_per_pair + 1
    local end_idx = i * keys_per_pair
    
    for j = start_idx, end_idx do
        local value = redis.call('GET', KEYS[j])
        if value then
            pair_total = pair_total + tonumber(value)
        end
    end
    
    table.insert(results, pair_total)
end

return results
"#;

/// Redis-based retry mapper trait providing high-performance retry tracking operations
#[async_trait]
pub trait RedisRetryMapper {
    /// Increment retry count for a token-customer pair on the current date
    /// 
    /// This operation is atomic and automatically sets a 31-day expiration on the key.
    /// Returns the new count after increment.
    async fn increment_retry_count(
        &self,
        token_id: &str,
        customer_id: CustomerId,
    ) -> CustomResult<u64, errors::StorageError>;

    /// Get rolling retry count for the specified number of days (default 30)
    /// 
    /// Returns aggregated count for the rolling window.
    /// Days are counted backwards from today (inclusive).
    /// Uses local date calculation + efficient Redis Lua script.
    async fn get_rolling_retry_count(
        &self,
        token_id: &str,
        customer_id: CustomerId,
        days: u8,
    ) -> CustomResult<RollingRetryCount, errors::StorageError>;

    /// Get retry count for a specific token-customer pair on a specific date
    /// 
    /// Date should be in yyyy-mm-dd format.
    /// This is a simple GET operation for a single key.
    async fn get_daily_retry_count(
        &self,
        token_id: &str,
        customer_id: CustomerId,
        date: &str,
    ) -> CustomResult<u64, errors::StorageError>;

    /// Generate retry keys for rolling window calculations (local computation)
    /// 
    /// Helper function that generates Redis keys for the last N days.
    /// This is computed locally for optimal performance.
    fn generate_retry_keys(&self, token_id: &str, customer_id: CustomerId, days: u8) -> Vec<String>;

    /// Get current date in yyyy-mm-dd format
    /// 
    /// Helper function for consistent date formatting.
    fn get_current_date(&self) -> String;
}

#[async_trait]
impl<T: DatabaseStore + Send + Sync> RedisRetryMapper for KVRouterStore<T> {
    #[instrument(skip_all)]
    async fn increment_retry_count(
        &self,
        token_id: &str,
        customer_id: CustomerId,
    ) -> CustomResult<u64, errors::StorageError> {
        let redis_conn = self
            .get_redis_conn()
            .change_context(errors::StorageError::KVError)?;

        let current_date = self.get_current_date();
        
        let count: u64 = redis_conn
            .evaluate_redis_script(
                INCREMENT_RETRY_COUNT_SCRIPT,
                Vec::<String>::new(), // No KEYS needed for this script
                vec![
                    token_id.to_string(),
                    customer_id.get_string_repr().to_string(),
                    current_date.clone(),
                ],
            )
            .await
            .change_context(errors::StorageError::KVError)?;

        tracing::info!(
            token_id = %token_id,
            customer_id = %customer_id.get_string_repr(),
            date = %current_date,
            new_count = %count,
            "Successfully incremented retry count"
        );

        Ok(count)
    }

    #[instrument(skip_all)]
    async fn get_rolling_retry_count(
        &self,
        token_id: &str,
        customer_id: CustomerId,
        days: u8,
    ) -> CustomResult<RollingRetryCount, errors::StorageError> {
        let redis_conn = self
            .get_redis_conn()
            .change_context(errors::StorageError::KVError)?;

        // Step 1: Generate retry keys locally (fast, no Redis calls)
        let retry_keys = self.generate_retry_keys(token_id, customer_id.clone(), days);
        
        // Step 2: Execute Lua script with pre-calculated keys for efficient server-side aggregation
        let total_count: u64 = redis_conn
            .evaluate_redis_script(
                GET_ROLLING_RETRY_COUNT_SCRIPT,
                retry_keys, // Pass as KEYS array to Lua script
                Vec::<String>::new(), // No ARGV needed
            )
            .await
            .change_context(errors::StorageError::KVError)?;

        let rolling_count = RollingRetryCount {
            token_id: token_id.to_string(),
            customer_id: customer_id.clone(),
            total_count,
            window_days: days,
        };

        tracing::info!(
            token_id = %token_id,
            customer_id = %customer_id.get_string_repr(),
            days = %days,
            total_count = %total_count,
            "Successfully retrieved rolling retry count"
        );

        Ok(rolling_count)
    }

    #[instrument(skip_all)]
    async fn get_daily_retry_count(
        &self,
        token_id: &str,
        customer_id: CustomerId,
        date: &str,
    ) -> CustomResult<u64, errors::StorageError> {
        let redis_conn = self
            .get_redis_conn()
            .change_context(errors::StorageError::KVError)?;

        let key = format!(
            "retry:{}:{}:{}",
            token_id,
            customer_id.get_string_repr(),
            date
        );

        let count: Option<u64> = redis_conn
            .get_key(&key.into())
            .await
            .map_err(|_| errors::StorageError::KVError)?;

        let count = count.unwrap_or(0);

        tracing::debug!(
            token_id = %token_id,
            customer_id = %customer_id.get_string_repr(),
            date = %date,
            count = %count,
            "Retrieved daily retry count"
        );

        Ok(count)
    }

    fn generate_retry_keys(&self, token_id: &str, customer_id: CustomerId, days: u8) -> Vec<String> {
        let mut keys = Vec::new();
        let today = date_time::now().date();
        
        // Generate keys for the last N days (today inclusive, going backwards)
        for i in 0..days {
            let date = today - Duration::days(i as i64);
            let key = format!(
                "retry:{}:{}:{:04}-{:02}-{:02}",
                token_id,
                customer_id.get_string_repr(),
                date.year(),
                date.month() as u8,
                date.day()
            );
            keys.push(key);
        }
        
        keys
    }

    fn get_current_date(&self) -> String {
        let today = date_time::now().date();
        format!("{:04}-{:02}-{:02}", today.year(), today.month() as u8, today.day())
    }
}
