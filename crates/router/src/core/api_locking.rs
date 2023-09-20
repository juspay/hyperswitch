use std::fmt::Debug;

use actix_web::rt::time as actix_time;
use error_stack::{IntoReport, ResultExt};
use redis_interface as redis;
use router_env::{instrument, logger, tracing};

use super::errors::{self, RouterResult};
use crate::routes::{app::AppStateInfo, lock_utils};

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
    // Queue it but return response as 2xx, could be used for webhooks
    QueueWithOk,
    // Ignore it and return response as 2xx, could be used for webhooks
    IgnoreWithOk,
    // Return Error
    Drop,
    // Locking Not applicable
    NotApplicable,
}

#[derive(Clone, Debug)]
pub struct LockingInput {
    pub unique_locking_key: String,
    pub api_identifier: lock_utils::ApiIdentifier,
}

impl LockingInput {
    fn get_redis_locking_key(&self, merchant_id: String) -> String {
        format!(
            "{}_{}_{}_{}",
            API_LOCK_PREFIX, merchant_id, self.api_identifier, self.unique_locking_key
        )
    }
}

impl LockAction {
    #[instrument(skip_all)]
    pub async fn perform_locking_action<A>(self, state: &A, merchant_id: String) -> RouterResult<()>
    where
        A: AppStateInfo,
    {
        match self {
            Self::Hold { input } => {
                let redis_conn = state
                    .store()
                    .get_redis_conn()
                    .change_context(errors::ApiErrorResponse::InternalServerError)?;

                let redis_locking_key = input.get_redis_locking_key(merchant_id);
                let delay_between_retries_in_milliseconds = state
                    .conf()
                    .lock_settings
                    .delay_between_retries_in_milliseconds;
                let redis_lock_expiry_seconds =
                    state.conf().lock_settings.redis_lock_expiry_seconds;
                let lock_retries = state.conf().lock_settings.lock_retries;
                for _retry in 0..lock_retries {
                    let redis_lock_result = redis_conn
                        .set_key_if_not_exists_with_expiry(
                            redis_locking_key.as_str(),
                            true, // [#2129] pick up request_id from AppState
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

                Err(errors::ApiErrorResponse::ResourceBusy).into_report()
            }
            Self::QueueWithOk | Self::IgnoreWithOk | Self::Drop | Self::NotApplicable => Ok(()),
        }
    }

    #[instrument(skip_all)]
    pub async fn free_lock_action<A>(self, state: &A, merchant_id: String) -> RouterResult<()>
    where
        A: AppStateInfo,
    {
        match self {
            Self::Hold { input } => {
                let redis_conn = state
                    .store()
                    .get_redis_conn()
                    .change_context(errors::ApiErrorResponse::InternalServerError)?;

                let redis_locking_key = input.get_redis_locking_key(merchant_id);
                // [#2129] Add a step to check whether the current lock is acquired by the current request and only then delete
                match redis_conn.delete_key(redis_locking_key.as_str()).await {
                    Ok(redis::types::DelReply::KeyDeleted) => {
                        logger::info!("Lock freed for locking input {:?}", input);
                        tracing::Span::current().record("redis_lock_released", redis_locking_key);
                        Ok(())
                    }
                    Ok(redis::types::DelReply::KeyNotDeleted) => {
                        Err(errors::ApiErrorResponse::InternalServerError)
                            .into_report()
                            .attach_printable(
                                "Status release lock called but key is not found in redis",
                            )
                    }
                    Err(error) => {
                        Err(error).change_context(errors::ApiErrorResponse::InternalServerError)
                    }
                }
            }
            Self::QueueWithOk | Self::IgnoreWithOk | Self::Drop | Self::NotApplicable => Ok(()),
        }
    }
}

pub trait GetLockingInput {
    fn get_locking_input<F>(&self, flow: F) -> LockAction
    where
        F: router_env::types::FlowMetric,
        lock_utils::ApiIdentifier: From<F>;
}
