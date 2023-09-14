use std::fmt::Debug;

use actix_web::rt::time as actix_time;
use error_stack::{IntoReport, ResultExt};
use redis_interface as redis;
use router_env::{instrument, logger, tracing};

use super::errors::{self, RouterResult};
use crate::routes::app::AppStateInfo;

pub const API_LOCK_PREFIX: &str = "API_LOCK";
pub const LOCK_RETRIES: u32 =
    ((REDIS_LOCK_EXPIRY_SECONDS * 1000) / DELAY_BETWEEN_RETRIES_IN_MILLISECONDS) - 1;
pub const REDIS_LOCK_EXPIRY_SECONDS: u32 = 60 * 3;
pub const DELAY_BETWEEN_RETRIES_IN_MILLISECONDS: u32 = 500;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LockStatus {
    // status when the lock is acquired by the caller
    Acquired, // [#2129] pick up request_id from AppState and populate here
    // status when the lock is acquired by some other caller
    Busy,
}

#[derive(Debug)]
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

#[derive(Debug)]
pub struct LockingInput {
    pub unique_locking_key: String,
    pub api_identifier: String,
    pub merchant_id: String,
}

impl LockingInput {
    fn get_redis_locking_key(&self) -> String {
        format!(
            "{}_{}_{}_{}",
            API_LOCK_PREFIX, self.merchant_id, self.api_identifier, self.unique_locking_key
        )
    }
}

impl LockAction {
    #[instrument(skip_all)]
    async fn perform_locking_action<A>(self, state: &A) -> RouterResult<()>
    where
        A: AppStateInfo,
    {
        match self {
            LockAction::Hold { input } => {
                let redis_conn = state
                    .store()
                    .get_redis_conn()
                    .change_context(errors::ApiErrorResponse::InternalServerError)?;

                let redis_locking_key = input.get_redis_locking_key();

                for _retry in 0..LOCK_RETRIES {
                    let redis_lock_result = redis_conn
                        .set_key_if_not_exists_with_expiry(
                            redis_locking_key.as_str(),
                            true, // [#2129] pick up request_id from AppState
                            Some(i64::from(REDIS_LOCK_EXPIRY_SECONDS)),
                        )
                        .await;

                    match redis_lock_result {
                        Ok(redis::SetnxReply::KeySet) => {
                            logger::info!("Lock acquired for locking input {:?}", input);
                            break;
                        }
                        Ok(redis::SetnxReply::KeyNotSet) => {
                            logger::info!(
                                "Lock busy by other request when tried for locking input {:?}",
                                input
                            );
                            actix_time::sleep(tokio::time::Duration::from_millis(u64::from(
                                DELAY_BETWEEN_RETRIES_IN_MILLISECONDS,
                            )))
                            .await;
                        }
                        Err(err) => {
                            return Err(err)
                                .change_context(errors::ApiErrorResponse::InternalServerError)
                        }
                    }
                }

                Ok(())
            }
            LockAction::QueueWithOk
            | LockAction::IgnoreWithOk
            | LockAction::Drop
            | LockAction::NotApplicable => Ok(()),
        }
    }

    #[instrument(skip_all)]
    async fn free_lock_action<A>(self, state: &A) -> RouterResult<Self>
    where
        A: AppStateInfo,
    {
        match self {
            LockAction::Hold { input } => {
                let redis_conn = state
                    .store()
                    .get_redis_conn()
                    .change_context(errors::ApiErrorResponse::InternalServerError)?;

                let redis_locking_key = input.get_redis_locking_key();
                // Add a step to check whether the current lock is acquired by the current request and only then delete
                match redis_conn.delete_key(redis_locking_key.as_str()).await {
                    Ok(redis::types::DelReply::KeyDeleted) => {
                        logger::info!("Lock freed for locking input {:?}", input);
                        Ok(LockAction::Hold { input })
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
            LockAction::QueueWithOk
            | LockAction::IgnoreWithOk
            | LockAction::Drop
            | LockAction::NotApplicable => Ok(self),
        }
    }
}

// pub trait GetLockAction {
//     // add generics for Flow and payload so that every combination of Flow and Payload implements this trait.
//     fn get_locking_action<T: Debug>(&self, _payload: &T) -> Option<LockAction> {
//         logger::warn!("Locking not enabled");
//         None
//     }
// }
