//! Core client session ID management for SDK authorization.

use common_utils::{
    errors::CustomResult,
    id_type::{self, GenerateId, MerchantId, PaymentId},
};
use error_stack::ResultExt;
use router_env::{instrument, logger, tracing};
use serde::{Deserialize, Serialize};
use time::{Duration, PrimitiveDateTime};

use crate::{consts, db::errors, routes::app::SessionStateInfo};

/// Client session data structure stored in Redis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientSessionData {
    /// SDK auth session ID
    pub client_session_id: id_type::ClientSessionId,
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

/// Manager for client session Redis operations
pub struct ClientSessionManager;

impl ClientSessionManager {
    /// Generate Redis key in format: client_session:{processor_merchant_id}:{payment_id}
    /// We have a unique constraint on (processor_merchant_id, payment_id), so using them
    /// for creating the redis key
    fn get_session_key(processor_merchant_id: &MerchantId, payment_id: &PaymentId) -> String {
        format!(
            "{}:{}:{}",
            consts::CLIENT_SESSION_KEY_PREFIX,
            processor_merchant_id.get_string_repr(),
            payment_id.get_string_repr()
        )
    }

    /// Create a new client session and store in Redis with TTL
    ///
    /// # Arguments
    /// * `state` - Application state with Redis connection
    /// * `processor_merchant_id` - Merchant ID for the payment
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
        processor_merchant_id: &MerchantId,
        payment_id: &PaymentId,
        session_expiry: Option<PrimitiveDateTime>,
    ) -> CustomResult<id_type::ClientSessionId, errors::ApiErrorResponse>
    where
        S: SessionStateInfo + Sync,
    {
        let redis_conn = state
            .store()
            .get_redis_conn()
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

        let session_expiry = session_expiry.unwrap_or_else(Self::get_default_session_expiry);

        // Generate a unique session ID
        let client_session_id = id_type::ClientSessionId::generate();

        let key = Self::get_session_key(processor_merchant_id, payment_id);

        // Calculate TTL in seconds from now until session_expiry
        let now = common_utils::date_time::now();
        let ttl_seconds = (session_expiry - now).whole_seconds();

        let ttl_seconds = if ttl_seconds > 0 {
            Ok(ttl_seconds)
        } else {
            Err(errors::ApiErrorResponse::PreconditionFailed {
                message: "Session expiry is in the past".to_string(),
            })
        }?;

        // Create session data structure
        let session_data = ClientSessionData {
            client_session_id: client_session_id.clone(),
            created_at: now,
            expires_at: session_expiry,
        };

        // Store session data as JSON with TTL
        redis_conn
            .serialize_and_set_key_with_expiry(&key.into(), &session_data, ttl_seconds)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

        logger::debug!(
            processor_merchant_id = %processor_merchant_id.get_string_repr(),
            payment_id = %payment_id.get_string_repr(),
            ttl_seconds,
            "Created client session"
        );

        Ok(client_session_id)
    }

    /// Get client session data from Redis
    ///
    /// # Arguments
    /// * `state` - Application state with Redis connection
    /// * `processor_merchant_id` - Merchant ID for the payment
    /// * `payment_id` - Payment ID for the session
    ///
    /// # Returns
    /// Session data if found, None if not found or expired
    #[instrument(skip_all)]
    pub async fn get_session<S>(
        state: &S,
        processor_merchant_id: &MerchantId,
        payment_id: &PaymentId,
    ) -> CustomResult<Option<ClientSessionData>, errors::ApiErrorResponse>
    where
        S: SessionStateInfo + Sync,
    {
        let redis_conn = state
            .store()
            .get_redis_conn()
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

        let key = Self::get_session_key(processor_merchant_id, payment_id);

        match redis_conn
            .get_and_deserialize_key::<ClientSessionData>(&key.into(), "ClientSessionData")
            .await
        {
            Ok(session_data) => {
                logger::debug!(
                    processor_merchant_id = %processor_merchant_id.get_string_repr(),
                    payment_id = %payment_id.get_string_repr(),
                    "Retrieved client session"
                );
                Ok(Some(session_data))
            }

            Err(err) => {
                if matches!(err.current_context(), errors::RedisError::NotFound) {
                    logger::debug!(
                        processor_merchant_id = %processor_merchant_id.get_string_repr(),
                        payment_id = %payment_id.get_string_repr(),
                        "No client session found"
                    );
                    Ok(None)
                } else {
                    Err(err).change_context(errors::ApiErrorResponse::InternalServerError)
                }
            }
        }
    }

    /// Validate session ID against stored value
    ///
    /// # Arguments
    /// * `state` - Application state with Redis connection
    /// * `processor_merchant_id` - Merchant ID for the payment
    /// * `payment_id` - Payment ID for the session
    /// * `session_id` - Session ID to validate
    ///
    /// # Returns
    /// `true` if session exists and session_id matches
    #[instrument(skip_all)]
    pub async fn validate_session<S>(
        state: &S,
        processor_merchant_id: &MerchantId,
        payment_id: &PaymentId,
        client_session_id: &id_type::ClientSessionId,
    ) -> CustomResult<bool, errors::ApiErrorResponse>
    where
        S: SessionStateInfo + Sync,
    {
        let session = Self::get_session(state, processor_merchant_id, payment_id)
            .await
            .attach_printable("Unable to retrieve client session")?;

        match session {
            Some(data) => {
                let is_valid = &data.client_session_id == client_session_id;
                if !is_valid {
                    logger::error!(
                        processor_merchant_id = %processor_merchant_id.get_string_repr(),
                        payment_id = %payment_id.get_string_repr(),
                        "Invalid client session ID"
                    );
                } else {
                    logger::debug!(
                        processor_merchant_id = %processor_merchant_id.get_string_repr(),
                        payment_id = %payment_id.get_string_repr(),
                        "Validated client session"
                    );
                }
                Ok(is_valid)
            }
            None => {
                logger::error!(
                    processor_merchant_id = %processor_merchant_id.get_string_repr(),
                    payment_id = %payment_id.get_string_repr(),
                    "Client session not found for validation"
                );
                Ok(false)
            }
        }
    }

    /// Recreate client session (invalidate old + create new)
    ///
    /// Convenience method that combines invalidate_session and create_session
    /// for payment update scenarios.
    ///
    /// # Arguments
    /// * `state` - Application state with Redis connection
    /// * `processor_merchant_id` - Merchant ID for the payment
    /// * `payment_id` - Payment ID for the session
    /// * `session_expiry` - Expiry time for the new session
    ///
    /// # Returns
    /// Tuple of (new_session_id, invalidation_report)
    #[instrument(skip_all)]
    pub async fn recreate_session<S>(
        state: &S,
        processor_merchant_id: &MerchantId,
        payment_id: &PaymentId,
        session_expiry: Option<PrimitiveDateTime>,
    ) -> CustomResult<(id_type::ClientSessionId, SessionInvalidationReport), errors::ApiErrorResponse>
    where
        S: SessionStateInfo + Sync,
    {
        let session_expiry = session_expiry.unwrap_or_else(Self::get_default_session_expiry);

        let session_existed = Self::get_session(state, processor_merchant_id, payment_id)
            .await?
            .is_some();

        let report = SessionInvalidationReport { session_existed };

        // Create new session, overwriting the previous value in redis
        let new_session_id = Self::create_session(
            state,
            processor_merchant_id,
            payment_id,
            Some(session_expiry),
        )
        .await?;

        logger::debug!(
            processor_merchant_id = %processor_merchant_id.get_string_repr(),
            payment_id = %payment_id.get_string_repr(),
            session_existed = report.session_existed,
            "Recreated client session"
        );

        Ok((new_session_id, report))
    }

    fn get_default_session_expiry() -> PrimitiveDateTime {
        common_utils::date_time::now() + Duration::seconds(consts::DEFAULT_SESSION_EXPIRY)
    }
}
