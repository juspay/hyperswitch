use actix_web::rt::time as actix_time;
use error_stack::{IntoReport, ResultExt};
use redis_interface as redis;
use router_env::logger;

use super::errors::{self, RouterResult};
use crate::routes;

// pr: should there be 2 enums AcquireLockStatus and ReleaseLockStatus ?
pub enum LockStatus {
    Acquired,
    AlreadyLocked,
    NotEnabled,
    AcquireFailedRetriesExhausted,
    Released,
    ReleaseFailedRetriesExhausted,
}

pub enum ActionOnWaitTimeout {}

pub struct LockingInput {
    pub unique_locking_key: String,
    pub api_identifier: String,
    pub action_on_wait_timeout: ActionOnWaitTimeout,
    pub merchant_id: String,
}

pub async fn get_key_and_lock_resource(
    state: &routes::AppState,
    locking_input: LockingInput,
    _should_retry: bool,
) -> RouterResult<LockStatus> {
    let api_identifier = locking_input.api_identifier.as_str();

    let is_locking_enabled_for_merchant = true;
    let is_locking_enabled_on_api = true;

    // let get_expiry_time = get_expiry_time_from_redis_based_on_connector_pmd_pm();
    if is_locking_enabled_for_merchant && is_locking_enabled_on_api {
        let expiry_in_seconds = 100; // get it from redis
        let delay_between_retries_in_seconds = 10; // get it from redis
        let retries = 1; // get from redis based on should_retry, if not present in redis default 1?
        let locking_key = locking_input.unique_locking_key;
        lock_resource(
            state,
            locking_key,
            expiry_in_seconds,
            retries,
            delay_between_retries_in_seconds,
            api_identifier,
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

pub async fn lock_resource(
    state: &routes::AppState,
    locking_key: String,
    expiry_in_seconds: u64,
    retries: i32,
    delay_between_retries_in_seconds: i64,
    _api_identifier: &str,
) -> RouterResult<LockStatus> {
    let redis_key_for_lock = get_redis_key_for_locks(locking_key);
    let redis_value_for_lock = true; // should get session id or request_id as we need info of who acquired the lock.
    acquire_lock_on_resource_in_redis(
        state,
        redis_key_for_lock.as_str(),
        redis_value_for_lock,
        expiry_in_seconds,
        delay_between_retries_in_seconds,
        retries,
    )
    .await
}

#[inline]
fn get_redis_key_for_locks(a: String) -> String {
    format!("SYNCHRONIZED_LOCK_{}", a)
}

pub async fn acquire_lock_on_resource_in_redis(
    state: &routes::AppState,
    key: &str,
    value: bool,
    expiry_in_seconds: u64,
    _delay_between_retries_in_seconds: i64,
    mut retries: i32,
) -> RouterResult<LockStatus> {
    let redis_conn = state.store.clone().get_redis_conn();

    while retries != 0 {
        // pr: should these be named as tries instead of retries
        retries -= 1;

        let is_lock_acquired = redis_conn
            .set_key_if_not_exists_with_expiry(
                key,
                value,
                Some(expiry_in_seconds
                    .try_into()
                    .into_report()
                    .change_context(errors::ApiErrorResponse::InternalServerError)?), // todo:  throw an appropriate error
            )
            .await;

        match is_lock_acquired {
            Ok(redis::SetnxReply::KeySet) => {
                // (addAquiredLockInfoToState redisKey >>= logLockAcquired)
                return Ok(LockStatus::Acquired);
            }
            Ok(redis::SetnxReply::KeyNotSet) => {
                logger::error!("Lock not acquired, retrying");
                actix_time::sleep(tokio::time::Duration::from_millis(expiry_in_seconds * 1000))
                    .await;
            }
            Err(error) => {
                logger::error!(error=%error.current_context(), "Error while locking");
                actix_time::sleep(tokio::time::Duration::from_millis(expiry_in_seconds * 1000))
                    .await;
            }
        }
    }

    Ok(LockStatus::AcquireFailedRetriesExhausted)
}

pub async fn release_lock(
    state: &routes::AppState,
    mut retries: i32,
    key: &str,
) -> RouterResult<LockStatus> {
    let redis_conn = state.store.clone().get_redis_conn();
    while retries != 0 {
        retries -= 1;

        match redis_conn.delete_key(key).await {
            Ok(_) => return Ok(LockStatus::Released),
            Err(error) => {
                logger::error!(error=%error.current_context(), "Error while locking");
            }
        }
    }
    Ok(LockStatus::ReleaseFailedRetriesExhausted)
}
