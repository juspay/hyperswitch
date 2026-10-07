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
use router_env::logger;

use super::MERCHANT_ID;
use crate::{
    core::{configs::dimension_state, errors, metrics},
    routes::SessionState,
    types::domain,
};

/// Checks every bucket and increments all of them only when none of them is full, so a rejected
/// webhook is not counted. The expiry is set whenever a bucket has none, which covers a new bucket
/// and one whose expiry was lost.
///
/// KEYS[i]: bucket keys, ARGV[1]: window in seconds, ARGV[i + 1]: limit for KEYS[i].
/// Returns 0 when allowed, otherwise the 1-based index of the first full bucket.
const CHECK_AND_INCREMENT_SCRIPT: &str = r#"
local window = tonumber(ARGV[1])
for _, key in ipairs(KEYS) do
  if redis.call('TTL', key) == -1 then
    redis.call('EXPIRE', key, window)
  end
end
for i, key in ipairs(KEYS) do
  local current = tonumber(redis.call('GET', key) or '0')
  if current >= tonumber(ARGV[i + 1]) then
    return i
  end
end
for _, key in ipairs(KEYS) do
  redis.call('INCR', key)
  if redis.call('TTL', key) < 0 then
    redis.call('EXPIRE', key, window)
  end
end
return 0
"#;

const REDIS_KEY_PREFIX: &str = "webhook_unverified_rl";

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
    let merchant_connector_id = merchant_connector_account.get_id();

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
            merchant_connector_id.clone(),
        );

    let storage = state.store.as_ref();
    let superposition_client = state.superposition_service.as_ref();
    let targeting_key = Some(merchant_id);

    let enabled = merchant_connector_account_dimensions
        .get_unverified_webhook_rate_limit_enabled(storage, superposition_client, targeting_key)
        .await;
    if !enabled {
        return Ok(());
    }

    // The window is shared by all buckets of a merchant, so it is resolved at the merchant level
    let window_in_secs = merchant_dimensions
        .get_unverified_webhook_rate_limit_window_in_secs(
            storage,
            superposition_client,
            targeting_key,
        )
        .await;
    let merchant_limit = merchant_dimensions
        .get_unverified_webhook_merchant_rate_limit(storage, superposition_client, targeting_key)
        .await;
    let profile_limit = profile_dimensions
        .get_unverified_webhook_profile_rate_limit(
            storage,
            superposition_client,
            Some(&merchant_connector_account.profile_id),
        )
        .await;
    let merchant_connector_account_limit = merchant_connector_account_dimensions
        .get_unverified_webhook_merchant_connector_account_rate_limit(
            storage,
            superposition_client,
            Some(&merchant_connector_id),
        )
        .await;

    // The merchant id is the hash tag of every key, so all buckets of a webhook map to the same
    // Redis Cluster slot, which a multi-key script requires. A limit of 0 disables its level.
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
        return Ok(());
    }

    metrics::WEBHOOK_UNVERIFIED_RATE_LIMIT_CHECKED_COUNT.add(
        1,
        router_env::metric_attributes!(
            (MERCHANT_ID, merchant_id.clone()),
            ("connector", connector_name.clone())
        ),
    );

    let exceeded_level = match check_and_increment(state, &buckets, window_in_secs).await {
        Ok(exceeded_level) => exceeded_level,
        Err(error) => {
            logger::error!(
                ?error,
                "Failed to check unverified webhook rate limit, letting the webhook through"
            );
            metrics::WEBHOOK_UNVERIFIED_RATE_LIMITER_ERROR_COUNT.add(
                1,
                router_env::metric_attributes!(
                    (MERCHANT_ID, merchant_id.clone()),
                    ("connector", connector_name.clone())
                ),
            );
            return Ok(());
        }
    };

    let Some(level) = exceeded_level else {
        return Ok(());
    };

    metrics::WEBHOOK_UNVERIFIED_RATE_LIMITED_COUNT.add(
        1,
        router_env::metric_attributes!(
            (MERCHANT_ID, merchant_id.clone()),
            ("connector", connector_name.clone()),
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

/// Runs the check-and-increment script and returns the level of the first full bucket, if any.
async fn check_and_increment(
    state: &SessionState,
    buckets: &[RateLimitBucket],
    window_in_secs: u32,
) -> errors::RouterResult<Option<RateLimitLevel>> {
    let redis_conn = state
        .store
        .get_redis_conn()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")?;

    // Scripts receive their keys as given, so the tenant prefix has to be added here.
    let keys = buckets
        .iter()
        .map(|bucket| redis_conn.add_prefix(&bucket.key))
        .collect::<Vec<_>>();
    let args = std::iter::once(window_in_secs.to_string())
        .chain(buckets.iter().map(|bucket| bucket.limit.to_string()))
        .collect::<Vec<_>>();

    let exceeded_index: i64 = redis_conn
        .evaluate_redis_script(CHECK_AND_INCREMENT_SCRIPT, keys, args)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to run the unverified webhook rate limit script")?;

    Ok(usize::try_from(exceeded_index)
        .ok()
        .and_then(|index| index.checked_sub(1))
        .and_then(|index| buckets.get(index))
        .map(|bucket| bucket.level))
}
