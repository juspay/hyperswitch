use router_env::logger;

use crate::{consts, core::metrics, routes::SessionState};

/// Fetches the global UCS kill switch error threshold from the DB config.
///
/// Returns 0 if the config key is absent or unparseable, meaning a single error
/// will trigger the kill switch.
async fn get_kill_switch_threshold(state: &SessionState) -> u64 {
    let db = state.store.as_ref();

    let config_result = db
        .find_config_by_key_unwrap_or(consts::UCS_KILL_SWITCH_THRESHOLD, Some("0".to_string()))
        .await;

    let config = match config_result {
        Ok(config) => config,
        Err(error) => {
            logger::error!(
                ?error,
                "Failed to fetch UCS kill switch threshold config from DB"
            );
            return 0;
        }
    };

    let threshold = config.config.parse::<u64>().unwrap_or_else(|error| {
        logger::error!(
            ?error,
            config_value = %config.config,
            "Failed to parse UCS kill switch threshold config, defaulting to 0"
        );
        0
    });

    threshold
}

/// Checks whether the kill switch is disabled for a specific config key
/// by looking up `ucs_kill_switch_disabled_{config_key}` in the DB config.
///
/// Returns true (disabled) only when the config value is explicitly "true".
/// Returns false (enabled) when the key is absent, unparseable, or "false".
async fn is_kill_switch_disabled(state: &SessionState, config_key: &str) -> bool {
    let db = state.store.as_ref();
    let disabled_key = format!("{}_{}", consts::UCS_KILL_SWITCH_DISABLED_PREFIX, config_key);

    let config_result = db
        .find_config_by_key_unwrap_or(&disabled_key, Some("false".to_string()))
        .await;

    let config = match config_result {
        Ok(config) => config,
        Err(error) => {
            logger::error!(
                ?error,
                disabled_key = %disabled_key,
                "Failed to fetch UCS kill switch disabled config from DB"
            );
            return false;
        }
    };

    let is_disabled = config.config.parse::<bool>().unwrap_or_else(|error| {
        logger::error!(
            ?error,
            disabled_key = %disabled_key,
            config_value = %config.config,
            "Failed to parse UCS kill switch disabled config, defaulting to false"
        );
        false
    });

    is_disabled
}

/// Fetches the current UCS error count from Redis for the given scope.
///
/// Returns 0 if the key does not exist or if the Redis call fails.
async fn get_error_count(state: &SessionState, redis_key: &str) -> u64 {
    let redis_conn = match state.store.get_redis_conn() {
        Ok(conn) => conn,
        Err(error) => {
            logger::error!(
                ?error,
                "Failed to get redis connection for UCS kill switch error count lookup"
            );
            return 0;
        }
    };

    let count_result = redis_conn
        .increment_fields_in_hash(&redis_key.into(), &[("count".to_string(), 0i64)])
        .await;

    let count = match count_result {
        Ok(counts) => counts.into_iter().next().unwrap_or(0) as u64,
        Err(error) => {
            logger::error!(
                ?error,
                redis_key = %redis_key,
                "Failed to get UCS kill switch error count from Redis"
            );
            0
        }
    };

    count
}

/// Increments the UCS error count in Redis for the given scope.
///
/// Uses HINCRBY to atomically increment. On the first error (count becomes 1),
/// sets a TTL of 1 day so the key auto-expires and UCS is retried.
///
/// Returns the count after increment.
async fn increment_error_count(state: &SessionState, redis_key: &str) -> u64 {
    let redis_conn = match state.store.get_redis_conn() {
        Ok(conn) => conn,
        Err(error) => {
            logger::error!(
                ?error,
                "Failed to get redis connection for UCS kill switch error recording"
            );
            return 0;
        }
    };

    let increment_result = redis_conn
        .increment_fields_in_hash(&redis_key.into(), &[("count".to_string(), 1i64)])
        .await;

    let new_count = match increment_result {
        Ok(counts) => counts.into_iter().next().unwrap_or(0) as u64,
        Err(error) => {
            logger::error!(
                ?error,
                redis_key = %redis_key,
                "Failed to increment UCS kill switch error count in Redis"
            );
            return 0;
        }
    };

    // Set TTL on first error so the key auto-expires after 1 day
    if new_count == 1 {
        let ttl_result = redis_conn
            .set_expiry(
                &redis_key.into(),
                consts::UCS_KILL_SWITCH_TTL_IN_SECS as i64,
            )
            .await;

        if let Err(error) = ttl_result {
            logger::error!(
                ?error,
                redis_key = %redis_key,
                "Failed to set TTL on UCS kill switch error count key"
            );
        }
    }

    new_count
}

/// Records a UCS error for kill switch evaluation.
///
/// Uses the matched rollout config key as scope, derives the Redis key, increments the count,
/// and emits a metric.
pub async fn record_ucs_error(state: &SessionState, matched_config_key: Option<&str>) {
    let config_key = match matched_config_key {
        Some(key) => key,
        None => {
            logger::debug!("No matched config key, skipping kill switch error recording");
            return;
        }
    };
    let redis_key = format!("{}_{}", consts::UCS_KILL_SWITCH_REDIS_PREFIX, config_key);
    let new_count = increment_error_count(state, &redis_key).await;

    logger::info!(
        matched_config_key = %config_key,
        new_count = new_count,
        "Recorded UCS error for kill switch"
    );

    metrics::UCS_KILL_SWITCH_ERROR_RECORDED.add(
        1,
        router_env::metric_attributes!(("matched_config_key", config_key.to_string()),),
    );
}

/// Determines whether the UCS kill switch should block UCS calls for a given scope.
///
/// Uses the matched rollout config key as scope for both the disabled check and the error
/// count check.
///
/// The kill switch is active when ALL of the following are true:
/// 1. The kill switch is not disabled for this scope via DB config
/// 2. The error count in Redis exceeds the configured threshold
///
/// Returns true if UCS should be bypassed in favor of the Direct path.
pub async fn should_force_direct_path(
    state: &SessionState,
    matched_config_key: Option<&str>,
) -> bool {
    let config_key = match matched_config_key {
        Some(key) => key,
        None => {
            logger::debug!("No matched config key, kill switch not applicable");
            return false;
        }
    };

    let is_disabled = is_kill_switch_disabled(state, config_key).await;
    if is_disabled {
        logger::info!(
            matched_config_key = %config_key,
            "UCS kill switch is disabled for this scope, allowing UCS"
        );
        return false;
    }

    let threshold = get_kill_switch_threshold(state).await;
    let redis_key = format!("{}_{}", consts::UCS_KILL_SWITCH_REDIS_PREFIX, config_key);
    let error_count = get_error_count(state, &redis_key).await;
    let is_active = error_count > threshold;

    logger::info!(
        matched_config_key = %config_key,
        error_count = error_count,
        threshold = threshold,
        is_active = is_active,
        "UCS kill switch evaluation"
    );

    if is_active {
        metrics::UCS_KILL_SWITCH_ACTIVATED.add(
            1,
            router_env::metric_attributes!(("matched_config_key", config_key.to_string()),),
        );
    }

    is_active
}
