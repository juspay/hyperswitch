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
                let state_ref = state.conf_as_ref();
                let delay_between_retries_in_milliseconds = state_ref
                    .lock_settings
                    .delay_between_retries_in_milliseconds
                    .to_owned();
                let redis_lock_expiry_seconds =
                    state_ref.lock_settings.redis_lock_expiry_seconds.to_owned();
                let lock_retries = input
                    .override_lock_retries
                    .unwrap_or(state_ref.lock_settings.lock_retries.to_owned());
                for _retry in 0..lock_retries {
                    let redis_lock_result = redis_conn
                        .set_key_if_not_exists_with_expiry(
                            redis_locking_key.as_str(),
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

                Err(errors::ApiErrorResponse::ResourceBusy).into_report()
            }
            Self::QueueWithOk | Self::Drop | Self::NotApplicable => Ok(()),
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

                match redis_conn
                    .get_key::<Option<String>>(&redis_locking_key)
                    .await
                {
                    Ok(val) => {
                        if val == state.get_request_id() {
                            match redis_conn.delete_key(redis_locking_key.as_str()).await {
                                Ok(redis::types::DelReply::KeyDeleted) => {
                                    logger::info!("Lock freed for locking input {:?}", input);
                                    tracing::Span::current()
                                        .record("redis_lock_released", redis_locking_key);
                                    Ok(())
                                }
                                Ok(redis::types::DelReply::KeyNotDeleted) => Err(
                                    errors::ApiErrorResponse::InternalServerError,
                                )
                                .into_report()
                                .attach_printable(
                                    "Status release lock called but key is not found in redis",
                                ),
                                Err(error) => Err(error)
                                    .change_context(errors::ApiErrorResponse::InternalServerError),
                            }
                        } else {
                            Err(errors::ApiErrorResponse::InternalServerError)
                                .into_report().attach_printable("The request_id which acquired the lock is not equal to the request_id requesting for releasing the lock")
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
