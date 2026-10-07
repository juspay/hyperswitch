//! Rate limiting for incoming webhooks whose source verification failed.
//!
//! An unverified webhook falls back to syncing with the connector (payment sync, refund sync,
//! payout retrieve, relay sync), so each one can trigger a connector call made with the
//! merchant's credentials. This caps how many unverified webhooks each merchant, profile and
//! merchant connector account can push through per window. Verified webhooks are never counted.
//!
//! Enablement, the window and the limit for each level are read from Superposition, so they can be
//! overridden per merchant, profile, connector or merchant connector account.

use std::str::FromStr;

use common_enums::connector_enums::Connector;
use error_stack::{report, ResultExt};
use router_env::{logger, tracing::Instrument};

use super::MERCHANT_ID;
use crate::{
    core::{configs::dimension_state, errors, metrics},
    routes::SessionState,
    types::domain,
};

const REDIS_KEY_PREFIX: &str = "webhook_rate_limit";

#[derive(Clone, Copy, Debug)]
enum RateLimitLevel {
    Merchant,
    Profile,
    MerchantConnectorAccount,
}

impl RateLimitLevel {
    fn as_str(self) -> &'static str {
        match self {
            Self::Merchant => "merchant",
            Self::Profile => "profile",
            Self::MerchantConnectorAccount => "merchant_connector_account",
        }
    }
}

struct RateLimitBucket {
    level: RateLimitLevel,
    key: String,
    limit: u32,
}

/// Counts an unverified webhook against the merchant, profile and merchant connector account
/// limits.
///
/// Returns `WebhookRateLimited` when enabled and one of the limits has been reached.
/// Any failure while checking the limits lets the webhook through.
pub async fn check_unverified_webhook_rate_limit(
    state: &SessionState,
    platform: &domain::Platform,
    merchant_connector_account: &domain::MerchantConnectorAccount,
) -> errors::RouterResult<()> {
    let merchant_id = platform.get_processor().get_account().get_id();
    let connector_name = merchant_connector_account.get_connector_name_as_string();

    // Each limit is resolved with only the dimensions of its own level, so an override set at a
    // narrower level can never change the limit of a bucket shared by a wider one
    let merchant_dimensions = dimension_state::Dimensions::new()
        .with_processor_merchant_id(platform.get_processor().get_processor_merchant_id())
        .with_provider_merchant_id(platform.get_provider().get_provider_merchant_id());
    let profile_dimensions =
        merchant_dimensions.with_profile_id(merchant_connector_account.profile_id.clone());
    let merchant_connector_account_dimensions =
        dimension_state::DimensionsWithMerchantConnectorAccount::new(
            profile_dimensions.clone(),
            Connector::from_str(&connector_name)
                .inspect_err(|error| {
                    logger::warn!(
                        ?error,
                        connector = %connector_name,
                        "Failed to parse connector for unverified webhook rate limit dimensions"
                    )
                })
                .ok(),
            merchant_connector_account.get_id(),
        );

    let enabled = merchant_connector_account_dimensions
        .get_unverified_webhook_rate_limit_enabled(
            state.store.as_ref(),
            state.superposition_service.as_ref(),
            Some(merchant_id),
        )
        .await;

    if enabled {
        let limits = RateLimitDimensions {
            merchant: merchant_dimensions,
            profile: profile_dimensions,
            merchant_connector_account: merchant_connector_account_dimensions,
        };
        count_against_limits(state, merchant_id, merchant_connector_account, &limits).await
    } else {
        Ok(())
    }
}

/// The dimensions each level's configs are resolved with.
struct RateLimitDimensions {
    merchant: dimension_state::DimensionsWithProcessorAndProviderMerchantId,
    profile: dimension_state::DimensionsWithProcessorAndProviderMerchantIdAndProfileId,
    merchant_connector_account: dimension_state::DimensionsWithMerchantConnectorAccount,
}

/// Resolves the window and limits, then counts the webhook against every level that has a limit.
async fn count_against_limits(
    state: &SessionState,
    merchant_id: &common_utils::id_type::MerchantId,
    merchant_connector_account: &domain::MerchantConnectorAccount,
    dimensions: &RateLimitDimensions,
) -> errors::RouterResult<()> {
    let connector_name = merchant_connector_account.get_connector_name_as_string();
    let merchant_connector_id = merchant_connector_account.get_id();
    let storage = state.store.as_ref();
    let superposition_client = state.superposition_service.as_ref();
    let targeting_key = Some(merchant_id);

    // The window is shared by all buckets of a merchant, so it is resolved at the merchant level
    let window_in_secs = dimensions
        .merchant
        .get_unverified_webhook_rate_limit_window_in_secs(
            storage,
            superposition_client,
            targeting_key,
        )
        .await;
    let merchant_limit = dimensions
        .merchant
        .get_unverified_webhook_merchant_rate_limit(storage, superposition_client, targeting_key)
        .await;
    let profile_limit = dimensions
        .profile
        .get_unverified_webhook_profile_rate_limit(
            storage,
            superposition_client,
            Some(&merchant_connector_account.profile_id),
        )
        .await;
    let merchant_connector_account_limit = dimensions
        .merchant_connector_account
        .get_unverified_webhook_merchant_connector_account_rate_limit(
            storage,
            superposition_client,
            Some(&merchant_connector_id),
        )
        .await;

    // The merchant id is the hash tag of every key, so all buckets of a webhook map to the same
    // Redis Cluster slot, which a multi-key transaction requires. A limit of 0 disables its level.
    let merchant_tag = format!("{REDIS_KEY_PREFIX}:{{{}}}", merchant_id.get_string_repr());
    let buckets: Vec<RateLimitBucket> = [
        (
            RateLimitLevel::Merchant,
            format!("{merchant_tag}:merchant"),
            merchant_limit,
        ),
        (
            RateLimitLevel::Profile,
            format!(
                "{merchant_tag}:profile:{}",
                merchant_connector_account.profile_id.get_string_repr()
            ),
            profile_limit,
        ),
        (
            RateLimitLevel::MerchantConnectorAccount,
            format!(
                "{merchant_tag}:mca:{}",
                merchant_connector_id.get_string_repr()
            ),
            merchant_connector_account_limit,
        ),
    ]
    .into_iter()
    .filter(|(_, _, limit)| *limit > 0)
    .map(|(level, key, limit)| RateLimitBucket { level, key, limit })
    .collect();

    if buckets.is_empty() || window_in_secs == 0 {
        Ok(())
    } else {
        metrics::WEBHOOK_UNVERIFIED_RATE_LIMIT_CHECKED_COUNT.add(
            1,
            router_env::metric_attributes!(
                (MERCHANT_ID, merchant_id.clone()),
                ("connector", connector_name.clone())
            ),
        );

        match increment_and_check(state, &buckets, window_in_secs).await {
            Ok(None) => Ok(()),
            Ok(Some(level)) => {
                metrics::WEBHOOK_UNVERIFIED_RATE_LIMITED_COUNT.add(
                    1,
                    router_env::metric_attributes!(
                        (MERCHANT_ID, merchant_id.clone()),
                        ("connector", connector_name),
                        ("level", level.as_str())
                    ),
                );
                logger::info!(
                    rate_limit_level = level.as_str(),
                    profile_id = %merchant_connector_account.profile_id.get_string_repr(),
                    merchant_connector_id = %merchant_connector_id.get_string_repr(),
                    "Unverified webhook rate limit reached"
                );

                Err(report!(errors::ApiErrorResponse::WebhookRateLimited))
            }
            Err(error) => {
                logger::error!(
                    ?error,
                    "Failed to check unverified webhook rate limit, letting the webhook through"
                );
                metrics::WEBHOOK_UNVERIFIED_RATE_LIMITER_ERROR_COUNT.add(
                    1,
                    router_env::metric_attributes!(
                        (MERCHANT_ID, merchant_id.clone()),
                        ("connector", connector_name)
                    ),
                );

                Ok(())
            }
        }
    }
}

/// Counts the webhook in every bucket and returns the level of the first bucket that went over
/// its limit, if any.
///
/// All buckets are incremented in one transaction, before any limit is checked. `Ok(None)` means
/// every bucket was still within its limit. `Ok(Some(level))` means the bucket of that level was
/// already full, in which case the webhook is rejected and a background task takes it back out
/// of every bucket, so that rejected webhooks do not use up the allowance.
///
/// Until that task has run, the counts are one too high. Another webhook checked in that moment
/// can be rejected although there was room for it, which only happens close to a limit.
async fn increment_and_check(
    state: &SessionState,
    buckets: &[RateLimitBucket],
    window_in_secs: u32,
) -> errors::RouterResult<Option<RateLimitLevel>> {
    let redis_conn = state
        .store
        .get_redis_conn()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")?;

    let keys = buckets
        .iter()
        .map(|bucket| redis_interface::RedisKey::from(&bucket.key))
        .collect::<Vec<_>>();
    let window_in_secs = i64::from(window_in_secs);

    // A bucket is created with the window as its expiry when the first webhook of a window is
    // counted, and later webhooks leave that expiry untouched, which makes the window fixed
    let counts = redis_conn
        .increment_keys_with_expiry(&keys, 1, window_in_secs)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to increment the unverified webhook rate limit buckets")?;

    // The count includes this webhook, so a bucket is over its limit only when the count is
    // greater than the limit
    let exceeded_level = buckets
        .iter()
        .zip(counts)
        .find(|(bucket, count)| *count > i64::from(bucket.limit))
        .map(|(bucket, _)| bucket.level);

    // A rejected webhook was already counted in every bucket by the increment above. Take it
    // back out with an increment of -1, so that rejected webhooks do not use up the allowance.
    // This runs in the background because the response does not need to wait for it
    if exceeded_level.is_some() {
        tokio::spawn(
            async move {
                redis_conn
                    .increment_keys_with_expiry(&keys, -1, window_in_secs)
                    .await
                    .inspect_err(|error| {
                        logger::error!(
                            ?error,
                            "Failed to take a rejected webhook out of the unverified webhook rate limit buckets"
                        )
                    })
                    .ok();
            }
            .in_current_span(),
        );
    }

    Ok(exceeded_level)
}
