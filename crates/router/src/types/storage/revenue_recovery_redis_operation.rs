use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};

use async_trait::async_trait;
use common_utils::{errors::CustomResult, id_type::CustomerId};
use error_stack::ResultExt;
use storage_impl::{
    redis::kv_store::{kv_wrapper, KvOperation, KvResult, PartitionKey, RedisConnInterface},
    kv_router_store::KVRouterStore,
    DatabaseStore,
};
// use storage_impl::redis_manipulation::*;
use router_env::{instrument, tracing};
use diesel_models::kv;

use crate::db::errors;

#[derive(Debug, Clone)]
pub struct PspTokenStatus {
    pub psp_token_id: String,
    pub locked_by_intent_id: String,
    pub error_code: String,
}

impl PspTokenStatus {
    pub fn is_locked(&self) -> bool {
        !self.locked_by_intent_id.is_empty()
    }
    
    pub fn is_success(&self) -> bool {
        self.error_code == "-1"
    }
}

// Lua script constants with improved variable names
const INSERT_MULTIPLE_PSP_TOKENS_SCRIPT: &str = r#"
local customer_id = KEYS[1]
local customer_psp_tokens_set_key = 'customer:' .. customer_id .. ':tokens'
local ttl_seconds = tonumber(ARGV[#ARGV - 1])
local default_error_code = ARGV[#ARGV]
local result = {}
local existing_psp_token_ids = redis.call('SMEMBERS', customer_psp_tokens_set_key)
local existing_psp_tokens_map = {}

-- Step 1: Lazy cleanup of expired PSP tokens
for _, psp_token_id in ipairs(existing_psp_token_ids) do
  local psp_token_hash_key = 'psp_token:' .. psp_token_id
  if redis.call('EXISTS', psp_token_hash_key) == 1 then
    existing_psp_tokens_map[psp_token_id] = true
  else
    -- Remove expired PSP token reference from customer set
    redis.call('SREM', customer_psp_tokens_set_key, psp_token_id)
  end
end

-- Step 2: Insert new PSP tokens if they don't exist
for i = 1, #ARGV - 2 do
  local new_psp_token_id = ARGV[i]
  if not existing_psp_tokens_map[new_psp_token_id] then
    local psp_token_hash_key = 'psp_token:' .. new_psp_token_id
    redis.call('HSET', psp_token_hash_key, 'locked_by_intent_id', '', 'error_code', default_error_code)
    redis.call('EXPIRE', psp_token_hash_key, ttl_seconds)
    redis.call('SADD', customer_psp_tokens_set_key, new_psp_token_id)
  end
end

-- Step 3: Refresh customer set TTL
redis.call('EXPIRE', customer_psp_tokens_set_key, ttl_seconds)

-- Step 4: Return all current PSP token information
local updated_psp_token_ids = redis.call('SMEMBERS', customer_psp_tokens_set_key)
for _, psp_token_id in ipairs(updated_psp_token_ids) do
  local psp_token_hash_key = 'psp_token:' .. psp_token_id
  local locked_by_intent_id = redis.call('HGET', psp_token_hash_key, 'locked_by_intent_id') or ''
  local error_code = redis.call('HGET', psp_token_hash_key, 'error_code') or ''
  table.insert(result, psp_token_id .. '|' .. locked_by_intent_id .. '|' .. error_code)
end

return result
"#;

const TRY_LOCK_PSP_TOKEN_SCRIPT: &str = r#"
local psp_token_hash_key = 'psp_token:' .. KEYS[1]
local intent_id = ARGV[1]
local current_locked_by_intent_id = redis.call('HGET', psp_token_hash_key, 'locked_by_intent_id')

if current_locked_by_intent_id == '' then
  redis.call('HSET', psp_token_hash_key, 'locked_by_intent_id', intent_id)
  return 'locked'
else
  return 'already_locked'
end
"#;

const UPDATE_AND_UNLOCK_PSP_TOKEN_SCRIPT: &str = r#"
local psp_token_hash_key = 'psp_token:' .. KEYS[1]
local provided_intent_id = ARGV[1]
local new_error_code = ARGV[2]
local stored_intent_id = redis.call('HGET', psp_token_hash_key, 'locked_by_intent_id')

if stored_intent_id == provided_intent_id then
  redis.call('HSET', psp_token_hash_key, 'locked_by_intent_id', '', 'error_code', new_error_code)
  return 'updated_and_unlocked'
else
  return 'intent_id_mismatch'
end
"#;

#[async_trait]
pub trait RedisPspTokenMap {
    async fn insert_multiple_psp_tokens(
        &self,
        customer_id: CustomerId,
        psp_token_ids: Vec<String>,
        ttl_seconds: i64,
        default_error_code: &str,
    ) -> CustomResult<Vec<PspTokenStatus>, errors::StorageError>;

    async fn try_lock_psp_token(
        &self,
        psp_token_id: &str,
        intent_id: &str,
    ) -> CustomResult<bool, errors::StorageError>;

    async fn update_and_unlock_psp_token(
        &self,
        psp_token_id: &str,
        intent_id: &str,
        error_code: &str,
    ) -> CustomResult<bool, errors::StorageError>;

    async fn get_best_psp_token_available(
        &self,
        customer_id: CustomerId,
        psp_token_list: Vec<String>,
        intent_id: &str,
    ) -> CustomResult<Option<PspTokenStatus>, errors::StorageError>;

    // Helper functions
    async fn call_decider_service(
        &self,
        available_psp_tokens: &[PspTokenStatus],
    ) -> CustomResult<Option<PspTokenStatus>, errors::StorageError>;

}

#[async_trait]
impl<T: DatabaseStore + Send + Sync> RedisPspTokenMap for KVRouterStore<T> {
    #[instrument(skip_all)]
    async fn insert_multiple_psp_tokens(
        &self,
        customer_id: CustomerId,
        psp_token_ids: Vec<String>,
        ttl_seconds: i64,
        default_error_code: &str,
    ) -> CustomResult<Vec<PspTokenStatus>, errors::StorageError> {
        let redis_conn = self.get_redis_conn()
            .change_context(errors::StorageError::KVError)?;

        let mut script_args = psp_token_ids.clone();
        script_args.push(ttl_seconds.to_string());
        script_args.push(default_error_code.to_string());

        let result: Vec<String> = redis_conn
            .evaluate_redis_script(
                INSERT_MULTIPLE_PSP_TOKENS_SCRIPT,
                vec![customer_id.get_string_repr().to_string()],
                script_args,
            )
            .await
            .change_context(errors::StorageError::KVError)?;

        let mut psp_tokens = Vec::new();
        for psp_token_str in result {
            let parts: Vec<&str> = psp_token_str.split('|').collect();
            if parts.len() == 3 {
                psp_tokens.push(PspTokenStatus {
                    psp_token_id: parts[0].to_string(),
                    locked_by_intent_id: parts[1].to_string(),
                    error_code: parts[2].to_string(),
                });
            }
        }

        tracing::info!(
            customer_id = %customer_id.get_string_repr(),
            psp_token_count = %psp_tokens.len(),
            ttl_seconds = %ttl_seconds,
            "Successfully inserted multiple PSP tokens"
        );

        Ok(psp_tokens)
    }

    #[instrument(skip_all)]
    async fn try_lock_psp_token(
        &self,
        psp_token_id: &str,
        intent_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let redis_conn = self.get_redis_conn()
            .change_context(errors::StorageError::KVError)?;

        let result: String = redis_conn
            .evaluate_redis_script(
                TRY_LOCK_PSP_TOKEN_SCRIPT,
                vec![psp_token_id.to_string()],
                vec![intent_id.to_string()],
            )
            .await
            .change_context(errors::StorageError::KVError)?;

        let is_locked = result == "locked";
        
        if is_locked {
            tracing::info!(
                psp_token_id = %psp_token_id,
                intent_id = %intent_id,
                "Successfully locked PSP token"
            );
        } else {
            tracing::warn!(
                psp_token_id = %psp_token_id,
                intent_id = %intent_id,
                result = %result,
                "Failed to lock PSP token"
            );
        }

        Ok(is_locked)
    }

    #[instrument(skip_all)]
    async fn update_and_unlock_psp_token(
        &self,
        psp_token_id: &str,
        intent_id: &str,
        error_code: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let redis_conn = self.get_redis_conn()
            .change_context(errors::StorageError::KVError)?;

        let result: String = redis_conn
            .evaluate_redis_script(
                UPDATE_AND_UNLOCK_PSP_TOKEN_SCRIPT,
                vec![psp_token_id.to_string()],
                vec![intent_id.to_string(), error_code.to_string()],
            )
            .await
            .change_context(errors::StorageError::KVError)?;

        let is_updated = result == "updated_and_unlocked";
        
        if is_updated {
            tracing::info!(
                psp_token_id = %psp_token_id,
                intent_id = %intent_id,
                error_code = %error_code,
                "Successfully updated and unlocked PSP token"
            );
        } else {
            tracing::warn!(
                psp_token_id = %psp_token_id,
                intent_id = %intent_id,
                result = %result,
                "Failed to update and unlock PSP token"
            );
        }

        Ok(is_updated)
    }

    #[instrument(skip_all)]
    async fn get_best_psp_token_available(
        &self,
        customer_id: CustomerId,
        psp_token_list: Vec<String>,
        intent_id: &str,
    ) -> CustomResult<Option<PspTokenStatus>, errors::StorageError> {
        // Step 1: Insert/update PSP tokens in Redis and get current state
        let current_psp_tokens = self.insert_multiple_psp_tokens(
            customer_id.clone(),
            psp_token_list,
            3600, // 1 hour TTL
            "0"   // default error code
        ).await?;

        // Step 2: Filter available (unlocked) PSP tokens
        let available_psp_tokens: Vec<PspTokenStatus> = current_psp_tokens
            .into_iter()
            .filter(|psp_token| !psp_token.is_locked())
            .collect();

        if available_psp_tokens.is_empty() {
            tracing::info!(
                customer_id = %customer_id.get_string_repr(),
                "No available PSP tokens found"
            );
            return Ok(None);
        }

        // Step 3: Local selection logic - Priority 1: Success tokens (error_code == "-1")
        let success_psp_tokens: Vec<&PspTokenStatus> = available_psp_tokens
            .iter()
            .filter(|psp_token| psp_token.is_success())
            .collect();

        let selected_psp_token = if !success_psp_tokens.is_empty() {
            // Use first success token
            success_psp_tokens[0].clone()
        } else {
            // Step 4: Priority 2: Use decider service for schedule-based selection
            match self.call_decider_service(&available_psp_tokens).await? {
                Some(best_psp_token) => best_psp_token,
                None => {
                    tracing::warn!(
                        customer_id = %customer_id.get_string_repr(),
                        "Decider service returned no token"
                    );
                    return Ok(None);
                }
            }
        };

        // Step 5: Try to lock the selected PSP token
        if self.try_lock_psp_token(&selected_psp_token.psp_token_id, intent_id).await? {
            let selection_method = if success_psp_tokens.is_empty() {
                "decider_service"
            } else {
                "success_psp_token"
            };

            tracing::info!(
                customer_id = %customer_id.get_string_repr(),
                psp_token_id = %selected_psp_token.psp_token_id,
                selection_method = %selection_method,
                "Successfully found and locked PSP token"
            );

            return Ok(Some(PspTokenStatus {
                psp_token_id: selected_psp_token.psp_token_id,
                locked_by_intent_id: intent_id.to_string(),
                error_code: selected_psp_token.error_code,
            }));
        } else {
            tracing::warn!(
                customer_id = %customer_id.get_string_repr(),
                psp_token_id = %selected_psp_token.psp_token_id,
                "Failed to lock selected PSP token - may have been locked by another process"
            );
        }

        Ok(None)
    }

    async fn call_decider_service(
        &self,
        available_psp_tokens: &[PspTokenStatus],
    ) -> CustomResult<Option<PspTokenStatus>, errors::StorageError> {
        // Dummy implementation - returns PSP token with earliest schedule time
        // In real implementation, this would call external service
        
        if available_psp_tokens.is_empty() {
            return Ok(None);
        }

        // Simulate schedule time calculation
        let best_psp_token = &available_psp_tokens[0];
        Ok(Some(best_psp_token.clone()))
    }
}
