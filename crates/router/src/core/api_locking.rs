use std::fmt::Debug;

use actix_web::rt::time as actix_time;
use error_stack::{report, ResultExt};
use redis_interface::{self as redis, RedisKey};
use router_env::{instrument, logger, tracing};

use super::errors::{self, RouterResult};
use crate::routes::{app::SessionStateInfo, lock_utils};

pub const API_LOCK_PREFIX: &str = "API_LOCK";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LockStatus {
    // status when the lock is acquired by the caller
    Acquired, // [#2129] pick up request_id from AppState and populate here
    // status when the lock is acquired by some other caller
    Busy,
}

#[derive(Clone, Debug)]
pub enum LockAction {
    // Sleep until the lock is acquired
    Hold { input: LockingInput },
    // Sleep until all locks are acquired
    HoldMultiple { inputs: Vec<LockingInput> },
    // Queue it but return response as 2xx, could be used for webhooks
    QueueWithOk,
    // Return Error
    Drop,
    // Locking Not applicable
    NotApplicable,
}

#[derive(Clone, Debug)]
pub struct LockingInput {
    pub unique_locking_key: String,
    pub api_identifier: lock_utils::ApiIdentifier,
    pub override_lock_retries: Option<u32>,
}

impl LockingInput {
    fn get_redis_locking_key(&self, merchant_id: &common_utils::id_type::MerchantId) -> String {
        format!(
            "{}_{}_{}_{}",
            API_LOCK_PREFIX,
            merchant_id.get_string_repr(),
            self.api_identifier,
            self.unique_locking_key
        )
    }
}

impl LockAction {
    #[instrument(skip_all)]
    pub async fn perform_locking_action<A>(
        self,
        state: &A,
        merchant_id: common_utils::id_type::MerchantId,
    ) -> RouterResult<()>
    where
        A: SessionStateInfo,
    {
        match self {
            Self::HoldMultiple { inputs } => {
                let lock_retries = inputs
                    .iter()
                    .find_map(|input| input.override_lock_retries)
                    .unwrap_or(state.conf().lock_settings.lock_retries);
                let request_id = state.get_request_id();
                let redis_lock_expiry_seconds =
                    state.conf().lock_settings.redis_lock_expiry_seconds;
                let redis_conn = state
                    .store()
                    .get_redis_conn()
                    .change_context(errors::ApiErrorResponse::InternalServerError)?;
                let redis_key_values = inputs
                    .iter()
                    .map(|input| input.get_redis_locking_key(&merchant_id))
                    .map(|key| (RedisKey::from(key.as_str()), request_id.clone()))
                    .collect::<Vec<_>>();
                let delay_between_retries_in_milliseconds = state
                    .conf()
                    .lock_settings
                    .delay_between_retries_in_milliseconds;
                for _retry in 0..lock_retries {
                    let results: Vec<redis::SetGetReply<_>> = redis_conn
                        .set_multiple_keys_if_not_exists_and_get_values(
                            &redis_key_values,
                            Some(i64::from(redis_lock_expiry_seconds)),
                        )
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)?;
                    let lock_acquired = results.iter().all(|res| {
                        // each redis value must match the request_id
                        // if even 1 does match, the lock is not acquired
                        *res.get_value() == request_id
                    });
                    if lock_acquired {
                        logger::info!("Lock acquired for locking inputs {:?}", inputs);
                        return Ok(());
                    } else {
                        actix_time::sleep(tokio::time::Duration::from_millis(u64::from(
                            delay_between_retries_in_milliseconds,
                        )))
                        .await;
                    }
                }
                Err(report!(errors::ApiErrorResponse::ResourceBusy))
            }
            Self::Hold { input } => {
                let redis_conn = state
                    .store()
                    .get_redis_conn()
                    .change_context(errors::ApiErrorResponse::InternalServerError)?;

                let redis_locking_key = input.get_redis_locking_key(&merchant_id);
                let delay_between_retries_in_milliseconds = state
                    .conf()
                    .lock_settings
                    .delay_between_retries_in_milliseconds;
                let redis_lock_expiry_seconds =
                    state.conf().lock_settings.redis_lock_expiry_seconds;
                let lock_retries = input
                    .override_lock_retries
                    .unwrap_or(state.conf().lock_settings.lock_retries);
                for _retry in 0..lock_retries {
                    let redis_lock_result = redis_conn
                        .set_key_if_not_exists_with_expiry(
                            &redis_locking_key.as_str().into(),
                            state.get_request_id(),
                            Some(i64::from(redis_lock_expiry_seconds)),
                        )
                        .await;

                    match redis_lock_result {
                        Ok(redis::SetnxReply::KeySet) => {
                            logger::info!("Lock acquired for locking input {:?}", input);
                            tracing::Span::current()
                                .record("redis_lock_acquired", redis_locking_key);
                            return Ok(());
                        }
                        Ok(redis::SetnxReply::KeyNotSet) => {
                            logger::info!(
                                "Lock busy by other request when tried for locking input {:?}",
                                input
                            );
                            actix_time::sleep(tokio::time::Duration::from_millis(u64::from(
                                delay_between_retries_in_milliseconds,
                            )))
                            .await;
                        }
                        Err(err) => {
                            return Err(err)
                                .change_context(errors::ApiErrorResponse::InternalServerError)
                        }
                    }
                }

                Err(report!(errors::ApiErrorResponse::ResourceBusy))
            }
            Self::QueueWithOk | Self::Drop | Self::NotApplicable => Ok(()),
        }
    }

    #[instrument(skip_all)]
    pub async fn free_lock_action<A>(
        self,
        state: &A,
        merchant_id: common_utils::id_type::MerchantId,
    ) -> RouterResult<()>
    where
        A: SessionStateInfo,
    {
        match self {
            Self::HoldMultiple { inputs } => {
                let redis_conn = state
                    .store()
                    .get_redis_conn()
                    .change_context(errors::ApiErrorResponse::InternalServerError)?;

                let redis_locking_keys = inputs
                    .iter()
                    .map(|input| RedisKey::from(input.get_redis_locking_key(&merchant_id).as_str()))
                    .collect::<Vec<_>>();
                let request_id = state.get_request_id();
                let values = redis_conn
                    .get_multiple_keys::<String>(&redis_locking_keys)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)?;

                let invalid_request_id_list = values
                    .iter()
                    .filter(|redis_value| **redis_value != request_id)
                    .flatten()
                    .collect::<Vec<_>>();

                if !invalid_request_id_list.is_empty() {
                    logger::error!(
                        "The request_id which acquired the lock is not equal to the request_id requesting for releasing the lock.
                        Current request_id: {:?},
                        Redis request_ids : {:?}",
                        request_id,
                        invalid_request_id_list
                    );
                    Err(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("The request_id which acquired the lock is not equal to the request_id requesting for releasing the lock")
                } else {
                    Ok(())
                }?;
                let delete_result = redis_conn
                    .delete_multiple_keys(&redis_locking_keys)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)?;
                let is_key_not_deleted = delete_result
                    .into_iter()
                    .any(|delete_reply| delete_reply.is_key_not_deleted());
                if is_key_not_deleted {
                    Err(errors::ApiErrorResponse::InternalServerError).attach_printable(
                        "Status release lock called but key is not found in redis",
                    )
                } else {
                    logger::info!("Lock freed for locking inputs {:?}", inputs);
                    Ok(())
                }
            }
            Self::Hold { input } => {
                let redis_conn = state
                    .store()
                    .get_redis_conn()
                    .change_context(errors::ApiErrorResponse::InternalServerError)?;

                let redis_locking_key = input.get_redis_locking_key(&merchant_id);

                match redis_conn
                    .get_key::<Option<String>>(&redis_locking_key.as_str().into())
                    .await
                {
                    Ok(val) => {
                        if val == state.get_request_id() {
                            match redis_conn
                                .delete_key(&redis_locking_key.as_str().into())
                                .await
                            {
                                Ok(redis::types::DelReply::KeyDeleted) => {
                                    logger::info!("Lock freed for locking input {:?}", input);
                                    tracing::Span::current()
                                        .record("redis_lock_released", redis_locking_key);
                                    Ok(())
                                }
                                Ok(redis::types::DelReply::KeyNotDeleted) => {
                                    Err(errors::ApiErrorResponse::InternalServerError)
                                        .attach_printable(
                                        "Status release lock called but key is not found in redis",
                                    )
                                }
                                Err(error) => Err(error)
                                    .change_context(errors::ApiErrorResponse::InternalServerError),
                            }
                        } else {
                            Err(errors::ApiErrorResponse::InternalServerError)
                                .attach_printable("The request_id which acquired the lock is not equal to the request_id requesting for releasing the lock")
                        }
                    }
                    Err(error) => {
                        Err(error).change_context(errors::ApiErrorResponse::InternalServerError)
                    }
                }
            }
            Self::QueueWithOk | Self::Drop | Self::NotApplicable => Ok(()),
        }
    }
}

pub trait GetLockingInput {
    fn get_locking_input<F>(&self, flow: F) -> LockAction
    where
        F: router_env::types::FlowMetric,
        lock_utils::ApiIdentifier: From<F>;
}
