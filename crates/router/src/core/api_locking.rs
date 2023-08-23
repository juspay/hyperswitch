use actix_web::rt::time as actix_time;
use error_stack::{IntoReport, ResultExt};
use redis_interface as redis;
use router_env::{instrument, logger, tracing};

use super::errors::{self, RouterResult};
use crate::routes;

#[derive(Debug)]
pub enum Lock {
    Acquired(String),
    Release(String),
}

pub struct LockingError {}

#[derive(Clone, Debug)]
pub enum LockStatus {
    Acquired(String),
    AlreadyLocked(String),
    NotEnabled,
    AcquireFailedRetriesExhausted(String),
    Released(String),
    ReleaseFailedRetriesExhausted(String),
}

#[derive(Debug)]
pub enum ActionOnWaitTimeout {
    Default,
}

#[derive(Debug)]
pub struct LockingInput {
    pub unique_locking_key: String,
    pub api_identifier: String,
    pub action_on_wait_timeout: ActionOnWaitTimeout,
    pub merchant_id: String,
}

impl LockingInput {
    fn get_redis_locking_key(self) -> String {
        format!(
            "API_LOCK_{}_{}_{}",
            self.merchant_id, self.api_identifier, self.unique_locking_key
        )
    }
}

#[instrument(skip(state))]
pub async fn get_key_and_lock_resource(
    state: &routes::AppState,
    locking_input: LockingInput,
    _should_retry: bool,
) -> RouterResult<LockStatus> {
    let is_locking_enabled_for_merchant = true;
    let is_locking_enabled_on_api = true;

    if is_locking_enabled_for_merchant && is_locking_enabled_on_api {
        let expiry_in_seconds = 60; // get it from redis
    
        let retries = 1; // get from redis based on should_retry, if not present in redis default 1?
        lock_resource(
            state,
            locking_input,
            expiry_in_seconds,
            retries,
         
        )
        .await
    } else {
        logger::info!(
            "Resource Locking not Enabled for merchant_id: {} and api: {}",
            locking_input.merchant_id,
            api_identifier.to_owned()
        );
        Ok(LockStatus::NotEnabled)
    }
}

#[instrument(skip(state))]
pub async fn lock_resource(
    state: &routes::AppState,
    locking_input: LockingInput,
    expiry_in_seconds: u64,
    retries: u32,
) -> RouterResult<LockStatus> {
    acquire_lock_on_resource_in_redis(
        state,
        locking_input.get_redis_key_for_locks(),
        true,
        expiry_in_seconds,
        delay_between_retries_in_milli_seconds,
        retries,
    )
    .await
}

#[instrument(skip(state))]
pub async fn acquire_lock_on_resource_in_redis(
    state: &routes::AppState,
    key: &str,
    value: bool,
    expiry_in_seconds: u64,
    delay_between_retries_in_milli_seconds: u64,
    mut retries: u32,
) -> RouterResult<LockStatus> {
    let redis_conn = state.store.clone().get_redis_conn();

    for _retry in 0..retries {
        let is_lock_acquired = redis_conn
            .set_key_if_not_exists_with_expiry(
                key,
                value,
                Some(
                    expiry_in_seconds
                        .try_into()
                        .into_report()
                        .change_context(errors::ApiErrorResponse::InternalServerError)?,
                ),
            )
            .await;

        match is_lock_acquired {
            Ok(redis::SetnxReply::KeySet) => {
                return Ok(LockStatus::Acquired(key.to_owned()));
            }
            Ok(redis::SetnxReply::KeyNotSet) => {
                logger::error!("Lock not acquired, retrying");
                actix_time::sleep(tokio::time::Duration::from_millis(
                    delay_between_retries_in_milli_seconds * 1000,
                ))
                .await;
            }
            Err(error) => {
                logger::error!(error=%error.current_context(), "Error while locking");
                actix_time::sleep(tokio::time::Duration::from_millis(
                    delay_between_retries_in_milli_seconds * 1000,
                ))
                .await;
            }
        }
    }

    Ok(LockStatus::AcquireFailedRetriesExhausted(key.to_owned()))
}

#[instrument(skip(state))]
pub async fn release_lock(
    state: &routes::AppState,
    mut retries: u32,
    lock: LockStatus,
) -> RouterResult<LockStatus> {
    let redis_conn = state.store.clone().get_redis_conn();

    match lock {
        LockStatus::Acquired(key) | LockStatus::AlreadyLocked(key) => {
            while retries != 0 {
                retries -= 1;

                match redis_conn.delete_key(key.as_str()).await {
                    Ok(_) => return Ok(LockStatus::Released(key.to_owned())),
                    Err(error) => {
                        logger::error!(error=%error.current_context(), "Error while releasing lock");
                    }
                }
            }
            Ok(LockStatus::ReleaseFailedRetriesExhausted(key.to_owned()))
        }
        LockStatus::NotEnabled => Ok(LockStatus::NotEnabled),
        LockStatus::AcquireFailedRetriesExhausted(key)
        | LockStatus::Released(key)
        | LockStatus::ReleaseFailedRetriesExhausted(key) => Ok(LockStatus::Released(key)),
    }
}

pub trait GetLockingInput {
    fn get_locking_input(&self) -> RouterResult<LockingInput>;
}

impl LockStatus {
    pub fn is_acquired(self) -> RouterResult<Self> {
        match self {
            a @ Self::Acquired(_) => Ok(a),
            b => Err(errors::ApiErrorResponse::InternalServerError)
                .into_report()
                .attach_printable(format!("Lock Status is not `Acquired` it is {:?}", b)),
        }
    }
}
