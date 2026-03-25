//! Payment Session Redis Operations - PR 1
//!
//! Core session ID management for SDK authorization.
//! PR 2 will add PM token tracking via session_entries.

use common_utils::{
    errors::CustomResult,
    id_type::{self, GenerateId, MerchantId, PaymentId},
};
use error_stack::ResultExt;
use redis_interface::DelReply;
use router_env::{instrument, logger, tracing};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{db::errors, routes::app::SessionStateInfo};

/// Payment session data structure stored in Redis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentSessionData {
    /// SDK auth session ID
    pub payment_session_id: id_type::PaymentSessionId,
    /// Session creation time
    pub created_at: PrimitiveDateTime,
    /// Session expiry time
    pub expires_at: PrimitiveDateTime,
}

/// Report of session invalidation results
#[derive(Debug)]
pub struct SessionInvalidationReport {
    /// Whether a session existed and was deleted
    pub session_existed: bool,
}

/// Manager for payment session Redis operations
pub struct PaymentSessionRedisManager;

impl PaymentSessionRedisManager {
    /// Generate Redis key in format: payment_session:{merchant_id}:{payment_id}
    fn get_session_key(merchant_id: &MerchantId, payment_id: &PaymentId) -> String {
        format!(
            "{}:{}:{}",
            crate::consts::PAYMENT_SESSION_KEY_PREFIX,
            merchant_id.get_string_repr(),
            payment_id.get_string_repr()
        )
    }

    /// Create a new payment session and store in Redis with TTL
    ///
    /// # Arguments
    /// * `state` - Application state with Redis connection
    /// * `merchant_id` - Merchant ID for the payment
    /// * `payment_id` - Payment ID for the session
    /// * `session_expiry` - Expiry time for the session
    ///
    /// # Returns
    /// The generated session ID string
    ///
    /// # Errors
    /// Returns error if Redis connection fails or TTL is invalid
    #[instrument(skip_all)]
    pub async fn create_session<S>(
        state: &S,
        merchant_id: &MerchantId,
        payment_id: &PaymentId,
        session_expiry: PrimitiveDateTime,
    ) -> CustomResult<id_type::PaymentSessionId, errors::StorageError>
    where
        S: SessionStateInfo + Sync,
    {
        let redis_conn =
            state
                .store()
                .get_redis_conn()
                .change_context(errors::StorageError::RedisError(
                    errors::RedisError::RedisConnectionError.into(),
                ))?;

        // Generate a unique session ID
        let payment_session_id = id_type::PaymentSessionId::generate();

        let key = Self::get_session_key(merchant_id, payment_id);

        // Calculate TTL in seconds from now until session_expiry
        let now = common_utils::date_time::now();
        let ttl_seconds = (session_expiry - now).whole_seconds();

        if ttl_seconds <= 0 {
            return Err(errors::StorageError::ValueNotFound(
                "Session expiry is in the past".to_string(),
            )
            .into());
        }

        // Create session data structure (PR 1: simplified, no session_entries)
        let session_data = PaymentSessionData {
            payment_session_id: payment_session_id.clone(),
            created_at: now,
            expires_at: session_expiry,
        };

        // Store session data as JSON with TTL
        redis_conn
            .serialize_and_set_key_with_expiry(&key.into(), &session_data, ttl_seconds)
            .await
            .change_context(errors::StorageError::RedisError(
                errors::RedisError::SetHashFieldFailed.into(),
            ))?;

        logger::debug!(
            merchant_id = %merchant_id.get_string_repr(),
            payment_id = %payment_id.get_string_repr(),
            ttl_seconds,
            "Created payment session"
        );

        Ok(payment_session_id)
    }

    /// Get payment session data from Redis
    ///
    /// # Arguments
    /// * `state` - Application state with Redis connection
    /// * `merchant_id` - Merchant ID for the payment
    /// * `payment_id` - Payment ID for the session
    ///
    /// # Returns
    /// Session data if found, None if not found or expired
    #[instrument(skip_all)]
    pub async fn get_session<S>(
        state: &S,
        merchant_id: &MerchantId,
        payment_id: &PaymentId,
    ) -> CustomResult<Option<PaymentSessionData>, errors::StorageError>
    where
        S: SessionStateInfo + Sync,
    {
        let redis_conn =
            state
                .store()
                .get_redis_conn()
                .change_context(errors::StorageError::RedisError(
                    errors::RedisError::RedisConnectionError.into(),
                ))?;

        let key = Self::get_session_key(merchant_id, payment_id);

        match redis_conn
            .get_and_deserialize_key::<PaymentSessionData>(&key.into(), "PaymentSessionData")
            .await
        {
            Ok(session_data) => {
                logger::debug!(
                    merchant_id = %merchant_id.get_string_repr(),
                    payment_id = %payment_id.get_string_repr(),
                    "Retrieved payment session"
                );
                Ok(Some(session_data))
            }
            Err(_) => {
                logger::debug!(
                    merchant_id = %merchant_id.get_string_repr(),
                    payment_id = %payment_id.get_string_repr(),
                    "No payment session found"
                );
                Ok(None)
            }
        }
    }

    /// Invalidate (delete) existing session for a payment
    ///
    /// # Arguments
    /// * `state` - Application state with Redis connection
    /// * `merchant_id` - Merchant ID for the payment
    /// * `payment_id` - Payment ID for the session
    ///
    /// # Returns
    /// Report indicating whether session existed
    #[instrument(skip_all)]
    async fn invalidate_session<S>(
        state: &S,
        merchant_id: &MerchantId,
        payment_id: &PaymentId,
    ) -> CustomResult<SessionInvalidationReport, errors::StorageError>
    where
        S: SessionStateInfo + Sync,
    {
        let redis_conn =
            state
                .store()
                .get_redis_conn()
                .change_context(errors::StorageError::RedisError(
                    errors::RedisError::RedisConnectionError.into(),
                ))?;

        let key = Self::get_session_key(merchant_id, payment_id);

        // Check if session exists before deleting
        let session_existed = Self::get_session(state, merchant_id, payment_id)
            .await?
            .is_some();

        // Delete payment session key
        match redis_conn.delete_key(&key.into()).await {
            Ok(DelReply::KeyDeleted) => {
                logger::debug!(
                    merchant_id = %merchant_id.get_string_repr(),
                    payment_id = %payment_id.get_string_repr(),
                    "Invalidated payment session"
                );
            }
            Ok(DelReply::KeyNotDeleted) => {
                logger::debug!(
                    merchant_id = %merchant_id.get_string_repr(),
                    payment_id = %payment_id.get_string_repr(),
                    "No payment session to invalidate"
                );
            }
            Err(err) => {
                logger::error!(?err, "Failed to delete payment session key");
            }
        }

        Ok(SessionInvalidationReport { session_existed })
    }

    /// Validate session ID against stored value
    ///
    /// # Arguments
    /// * `state` - Application state with Redis connection
    /// * `merchant_id` - Merchant ID for the payment
    /// * `payment_id` - Payment ID for the session
    /// * `session_id` - Session ID to validate
    ///
    /// # Returns
    /// `true` if session exists and session_id matches
    #[instrument(skip_all)]
    pub async fn validate_session<S>(
        state: &S,
        merchant_id: &MerchantId,
        payment_id: &PaymentId,
        payment_session_id: &id_type::PaymentSessionId,
    ) -> CustomResult<bool, errors::StorageError>
    where
        S: SessionStateInfo + Sync,
    {
        match Self::get_session(state, merchant_id, payment_id).await? {
            Some(data) => {
                let is_valid = &data.payment_session_id == payment_session_id;
                logger::debug!(
                    merchant_id = %merchant_id.get_string_repr(),
                    payment_id = %payment_id.get_string_repr(),
                    is_valid,
                    "Validated payment session"
                );
                Ok(is_valid)
            }
            None => {
                logger::debug!(
                    merchant_id = %merchant_id.get_string_repr(),
                    payment_id = %payment_id.get_string_repr(),
                    "Payment session not found for validation"
                );
                Ok(false)
            }
        }
    }

    /// Recreate payment session (invalidate old + create new)
    ///
    /// Convenience method that combines invalidate_session and create_session
    /// for payment update scenarios.
    ///
    /// # Arguments
    /// * `state` - Application state with Redis connection
    /// * `merchant_id` - Merchant ID for the payment
    /// * `payment_id` - Payment ID for the session
    /// * `session_expiry` - Expiry time for the new session
    ///
    /// # Returns
    /// Tuple of (new_session_id, invalidation_report)
    #[instrument(skip_all)]
    pub async fn recreate_session<S>(
        state: &S,
        merchant_id: &MerchantId,
        payment_id: &PaymentId,
        session_expiry: PrimitiveDateTime,
    ) -> CustomResult<(id_type::PaymentSessionId, SessionInvalidationReport), errors::StorageError>
    where
        S: SessionStateInfo + Sync,
    {
        // Invalidate old session
        let report = Self::invalidate_session(state, merchant_id, payment_id).await?;

        // Create new session
        let new_session_id =
            Self::create_session(state, merchant_id, payment_id, session_expiry).await?;

        logger::debug!(
            merchant_id = %merchant_id.get_string_repr(),
            payment_id = %payment_id.get_string_repr(),
            session_existed = report.session_existed,
            "Recreated payment session"
        );

        Ok((new_session_id, report))
    }
}
