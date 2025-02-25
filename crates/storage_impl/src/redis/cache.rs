use std::{any::Any, borrow::Cow, fmt::Debug, sync::Arc};

use common_utils::{
    errors::{self, CustomResult},
    ext_traits::ByteSliceExt,
};
use dyn_clone::DynClone;
use error_stack::{Report, ResultExt};
use moka::future::Cache as MokaCache;
use once_cell::sync::Lazy;
use redis_interface::{errors::RedisError, RedisConnectionPool, RedisValue};
use router_env::{
    logger,
    tracing::{self, instrument},
};

use crate::{
    errors::StorageError,
    metrics,
    redis::{PubSubInterface, RedisConnInterface},
};

/// Redis channel name used for publishing invalidation messages
pub const IMC_INVALIDATION_CHANNEL: &str = "hyperswitch_invalidate";

/// Time to live 30 mins
const CACHE_TTL: u64 = 30 * 60;

/// Time to idle 10 mins
const CACHE_TTI: u64 = 10 * 60;

/// Max Capacity of Cache in MB
const MAX_CAPACITY: u64 = 30;

/// Config Cache with time_to_live as 30 mins and time_to_idle as 10 mins.
pub static CONFIG_CACHE: Lazy<Cache> =
    Lazy::new(|| Cache::new("CONFIG_CACHE", CACHE_TTL, CACHE_TTI, None));

/// Accounts cache with time_to_live as 30 mins and size limit
pub static ACCOUNTS_CACHE: Lazy<Cache> =
    Lazy::new(|| Cache::new("ACCOUNTS_CACHE", CACHE_TTL, CACHE_TTI, Some(MAX_CAPACITY)));

/// Routing Cache
pub static ROUTING_CACHE: Lazy<Cache> =
    Lazy::new(|| Cache::new("ROUTING_CACHE", CACHE_TTL, CACHE_TTI, Some(MAX_CAPACITY)));

/// 3DS Decision Manager Cache
pub static DECISION_MANAGER_CACHE: Lazy<Cache> = Lazy::new(|| {
    Cache::new(
        "DECISION_MANAGER_CACHE",
        CACHE_TTL,
        CACHE_TTI,
        Some(MAX_CAPACITY),
    )
});

/// Surcharge Cache
pub static SURCHARGE_CACHE: Lazy<Cache> =
    Lazy::new(|| Cache::new("SURCHARGE_CACHE", CACHE_TTL, CACHE_TTI, Some(MAX_CAPACITY)));

/// CGraph Cache
pub static CGRAPH_CACHE: Lazy<Cache> =
    Lazy::new(|| Cache::new("CGRAPH_CACHE", CACHE_TTL, CACHE_TTI, Some(MAX_CAPACITY)));

/// PM Filter CGraph Cache
pub static PM_FILTERS_CGRAPH_CACHE: Lazy<Cache> = Lazy::new(|| {
    Cache::new(
        "PM_FILTERS_CGRAPH_CACHE",
        CACHE_TTL,
        CACHE_TTI,
        Some(MAX_CAPACITY),
    )
});

/// Success based Dynamic Algorithm Cache
pub static SUCCESS_BASED_DYNAMIC_ALGORITHM_CACHE: Lazy<Cache> = Lazy::new(|| {
    Cache::new(
        "SUCCESS_BASED_DYNAMIC_ALGORITHM_CACHE",
        CACHE_TTL,
        CACHE_TTI,
        Some(MAX_CAPACITY),
    )
});

/// Elimination based Dynamic Algorithm Cache
pub static ELIMINATION_BASED_DYNAMIC_ALGORITHM_CACHE: Lazy<Cache> = Lazy::new(|| {
    Cache::new(
        "ELIMINATION_BASED_DYNAMIC_ALGORITHM_CACHE",
        CACHE_TTL,
        CACHE_TTI,
        Some(MAX_CAPACITY),
    )
});

/// Contract Routing based Dynamic Algorithm Cache
pub static CONTRACT_BASED_DYNAMIC_ALGORITHM_CACHE: Lazy<Cache> = Lazy::new(|| {
    Cache::new(
        "CONTRACT_BASED_DYNAMIC_ALGORITHM_CACHE",
        CACHE_TTL,
        CACHE_TTI,
        Some(MAX_CAPACITY),
    )
});

/// Trait which defines the behaviour of types that's gonna be stored in Cache
pub trait Cacheable: Any + Send + Sync + DynClone {
    fn as_any(&self) -> &dyn Any;
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CacheRedact<'a> {
    pub tenant: String,
    pub kind: CacheKind<'a>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum CacheKind<'a> {
    Config(Cow<'a, str>),
    Accounts(Cow<'a, str>),
    Routing(Cow<'a, str>),
    DecisionManager(Cow<'a, str>),
    Surcharge(Cow<'a, str>),
    CGraph(Cow<'a, str>),
    SuccessBasedDynamicRoutingCache(Cow<'a, str>),
    EliminationBasedDynamicRoutingCache(Cow<'a, str>),
    ContractBasedDynamicRoutingCache(Cow<'a, str>),
    PmFiltersCGraph(Cow<'a, str>),
    All(Cow<'a, str>),
}

impl CacheKind<'_> {
    pub(crate) fn get_key_without_prefix(&self) -> &str {
        match self {
            CacheKind::Config(key)
            | CacheKind::Accounts(key)
            | CacheKind::Routing(key)
            | CacheKind::DecisionManager(key)
            | CacheKind::Surcharge(key)
            | CacheKind::CGraph(key)
            | CacheKind::SuccessBasedDynamicRoutingCache(key)
            | CacheKind::EliminationBasedDynamicRoutingCache(key)
            | CacheKind::ContractBasedDynamicRoutingCache(key)
            | CacheKind::PmFiltersCGraph(key)
            | CacheKind::All(key) => key,
        }
    }
}

impl<'a> TryFrom<CacheRedact<'a>> for RedisValue {
    type Error = Report<errors::ValidationError>;
    fn try_from(v: CacheRedact<'a>) -> Result<Self, Self::Error> {
        Ok(Self::from_bytes(serde_json::to_vec(&v).change_context(
            errors::ValidationError::InvalidValue {
                message: "Invalid publish key provided in pubsub".into(),
            },
        )?))
    }
}

impl TryFrom<RedisValue> for CacheRedact<'_> {
    type Error = Report<errors::ValidationError>;

    fn try_from(v: RedisValue) -> Result<Self, Self::Error> {
        let bytes = v.as_bytes().ok_or(errors::ValidationError::InvalidValue {
            message: "InvalidValue received in pubsub".to_string(),
        })?;

        bytes
            .parse_struct("CacheRedact")
            .change_context(errors::ValidationError::InvalidValue {
                message: "Unable to deserialize the value from pubsub".to_string(),
            })
    }
}

impl<T> Cacheable for T
where
    T: Any + Clone + Send + Sync,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
}

dyn_clone::clone_trait_object!(Cacheable);

pub struct Cache {
    name: &'static str,
    inner: MokaCache<String, Arc<dyn Cacheable>>,
}

#[derive(Debug, Clone)]
pub struct CacheKey {
    pub key: String,
    // #TODO: make it usage specific enum Eg: CacheKind { Tenant(String), NoTenant, Partition(String) }
    pub prefix: String,
}

impl From<CacheKey> for String {
    fn from(val: CacheKey) -> Self {
        if val.prefix.is_empty() {
            val.key
        } else {
            format!("{}:{}", val.prefix, val.key)
        }
    }
}

impl Cache {
    /// With given `time_to_live` and `time_to_idle` creates a moka cache.
    ///
    /// `name`        : Cache type name to be used as an attribute in metrics
    /// `time_to_live`: Time in seconds before an object is stored in a caching system before it’s deleted
    /// `time_to_idle`: Time in seconds before a `get` or `insert` operation an object is stored in a caching system before it's deleted
    /// `max_capacity`: Max size in MB's that the cache can hold
    pub fn new(
        name: &'static str,
        time_to_live: u64,
        time_to_idle: u64,
        max_capacity: Option<u64>,
    ) -> Self {
        // Record the metrics of manual invalidation of cache entry by the application
        let eviction_listener = move |_, _, cause| {
            metrics::IN_MEMORY_CACHE_EVICTION_COUNT.add(
                1,
                router_env::metric_attributes!(
                    ("cache_type", name.to_owned()),
                    ("removal_cause", format!("{:?}", cause)),
                ),
            );
        };
        let mut cache_builder = MokaCache::builder()
            .time_to_live(std::time::Duration::from_secs(time_to_live))
            .time_to_idle(std::time::Duration::from_secs(time_to_idle))
            .eviction_listener(eviction_listener);

        if let Some(capacity) = max_capacity {
            cache_builder = cache_builder.max_capacity(capacity * 1024 * 1024);
        }

        Self {
            name,
            inner: cache_builder.build(),
        }
    }

    pub async fn push<T: Cacheable>(&self, key: CacheKey, val: T) {
        self.inner.insert(key.into(), Arc::new(val)).await;
    }

    pub async fn get_val<T: Clone + Cacheable>(&self, key: CacheKey) -> Option<T> {
        let val = self.inner.get::<String>(&key.into()).await;

        // Add cache hit and cache miss metrics
        if val.is_some() {
            metrics::IN_MEMORY_CACHE_HIT
                .add(1, router_env::metric_attributes!(("cache_type", self.name)));
        } else {
            metrics::IN_MEMORY_CACHE_MISS
                .add(1, router_env::metric_attributes!(("cache_type", self.name)));
        }

        let val = (*val?).as_any().downcast_ref::<T>().cloned();

        val
    }

    /// Check if a key exists in cache
    pub async fn exists(&self, key: CacheKey) -> bool {
        self.inner.contains_key::<String>(&key.into())
    }

    pub async fn remove(&self, key: CacheKey) {
        self.inner.invalidate::<String>(&key.into()).await;
    }

    /// Performs any pending maintenance operations needed by the cache.
    async fn run_pending_tasks(&self) {
        self.inner.run_pending_tasks().await;
    }

    /// Returns an approximate number of entries in this cache.
    fn get_entry_count(&self) -> u64 {
        self.inner.entry_count()
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub async fn record_entry_count_metric(&self) {
        self.run_pending_tasks().await;

        metrics::IN_MEMORY_CACHE_ENTRY_COUNT.record(
            self.get_entry_count(),
            router_env::metric_attributes!(("cache_type", self.name)),
        );
    }
}

#[instrument(skip_all)]
pub async fn get_or_populate_redis<T, F, Fut>(
    redis: &Arc<RedisConnectionPool>,
    key: impl AsRef<str>,
    fun: F,
) -> CustomResult<T, StorageError>
where
    T: serde::Serialize + serde::de::DeserializeOwned + Debug,
    F: FnOnce() -> Fut + Send,
    Fut: futures::Future<Output = CustomResult<T, StorageError>> + Send,
{
    let type_name = std::any::type_name::<T>();
    let key = key.as_ref();
    let redis_val = redis
        .get_and_deserialize_key::<T>(&key.into(), type_name)
        .await;
    let get_data_set_redis = || async {
        let data = fun().await?;
        redis
            .serialize_and_set_key(&key.into(), &data)
            .await
            .change_context(StorageError::KVError)?;
        Ok::<_, Report<StorageError>>(data)
    };
    match redis_val {
        Err(err) => match err.current_context() {
            RedisError::NotFound | RedisError::JsonDeserializationFailed => {
                get_data_set_redis().await
            }
            _ => Err(err
                .change_context(StorageError::KVError)
                .attach_printable(format!("Error while fetching cache for {type_name}"))),
        },
        Ok(val) => Ok(val),
    }
}

#[instrument(skip_all)]
pub async fn get_or_populate_in_memory<T, F, Fut>(
    store: &(dyn RedisConnInterface + Send + Sync),
    key: &str,
    fun: F,
    cache: &Cache,
) -> CustomResult<T, StorageError>
where
    T: Cacheable + serde::Serialize + serde::de::DeserializeOwned + Debug + Clone,
    F: FnOnce() -> Fut + Send,
    Fut: futures::Future<Output = CustomResult<T, StorageError>> + Send,
{
    let redis = &store
        .get_redis_conn()
        .change_context(StorageError::RedisError(
            RedisError::RedisConnectionError.into(),
        ))
        .attach_printable("Failed to get redis connection")?;
    let cache_val = cache
        .get_val::<T>(CacheKey {
            key: key.to_string(),
            prefix: redis.key_prefix.clone(),
        })
        .await;
    if let Some(val) = cache_val {
        Ok(val)
    } else {
        let val = get_or_populate_redis(redis, key, fun).await?;
        cache
            .push(
                CacheKey {
                    key: key.to_string(),
                    prefix: redis.key_prefix.clone(),
                },
                val.clone(),
            )
            .await;
        Ok(val)
    }
}

#[instrument(skip_all)]
pub async fn redact_from_redis_and_publish<
    'a,
    K: IntoIterator<Item = CacheKind<'a>> + Send + Clone,
>(
    store: &(dyn RedisConnInterface + Send + Sync),
    keys: K,
) -> CustomResult<usize, StorageError> {
    let redis_conn = store
        .get_redis_conn()
        .change_context(StorageError::RedisError(
            RedisError::RedisConnectionError.into(),
        ))
        .attach_printable("Failed to get redis connection")?;

    let redis_keys_to_be_deleted = keys
        .clone()
        .into_iter()
        .map(|val| val.get_key_without_prefix().to_owned().into())
        .collect::<Vec<_>>();

    let del_replies = redis_conn
        .delete_multiple_keys(&redis_keys_to_be_deleted)
        .await
        .map_err(StorageError::RedisError)?;

    let deletion_result = redis_keys_to_be_deleted
        .into_iter()
        .zip(del_replies)
        .collect::<Vec<_>>();

    logger::debug!(redis_deletion_result=?deletion_result);

    let futures = keys.into_iter().map(|key| async {
        redis_conn
            .clone()
            .publish(IMC_INVALIDATION_CHANNEL, key)
            .await
            .change_context(StorageError::KVError)
    });

    Ok(futures::future::try_join_all(futures)
        .await?
        .iter()
        .sum::<usize>())
}

#[instrument(skip_all)]
pub async fn publish_and_redact<'a, T, F, Fut>(
    store: &(dyn RedisConnInterface + Send + Sync),
    key: CacheKind<'a>,
    fun: F,
) -> CustomResult<T, StorageError>
where
    F: FnOnce() -> Fut + Send,
    Fut: futures::Future<Output = CustomResult<T, StorageError>> + Send,
{
    let data = fun().await?;
    redact_from_redis_and_publish(store, [key]).await?;
    Ok(data)
}

#[instrument(skip_all)]
pub async fn publish_and_redact_multiple<'a, T, F, Fut, K>(
    store: &(dyn RedisConnInterface + Send + Sync),
    keys: K,
    fun: F,
) -> CustomResult<T, StorageError>
where
    F: FnOnce() -> Fut + Send,
    Fut: futures::Future<Output = CustomResult<T, StorageError>> + Send,
    K: IntoIterator<Item = CacheKind<'a>> + Send + Clone,
{
    let data = fun().await?;
    redact_from_redis_and_publish(store, keys).await?;
    Ok(data)
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    #[tokio::test]
    async fn construct_and_get_cache() {
        let cache = Cache::new("test", 1800, 1800, None);
        cache
            .push(
                CacheKey {
                    key: "key".to_string(),
                    prefix: "prefix".to_string(),
                },
                "val".to_string(),
            )
            .await;
        assert_eq!(
            cache
                .get_val::<String>(CacheKey {
                    key: "key".to_string(),
                    prefix: "prefix".to_string()
                })
                .await,
            Some(String::from("val"))
        );
    }

    #[tokio::test]
    async fn eviction_on_size_test() {
        let cache = Cache::new("test", 2, 2, Some(0));
        cache
            .push(
                CacheKey {
                    key: "key".to_string(),
                    prefix: "prefix".to_string(),
                },
                "val".to_string(),
            )
            .await;
        assert_eq!(
            cache
                .get_val::<String>(CacheKey {
                    key: "key".to_string(),
                    prefix: "prefix".to_string()
                })
                .await,
            None
        );
    }

    #[tokio::test]
    async fn invalidate_cache_for_key() {
        let cache = Cache::new("test", 1800, 1800, None);
        cache
            .push(
                CacheKey {
                    key: "key".to_string(),
                    prefix: "prefix".to_string(),
                },
                "val".to_string(),
            )
            .await;

        cache
            .remove(CacheKey {
                key: "key".to_string(),
                prefix: "prefix".to_string(),
            })
            .await;

        assert_eq!(
            cache
                .get_val::<String>(CacheKey {
                    key: "key".to_string(),
                    prefix: "prefix".to_string()
                })
                .await,
            None
        );
    }

    #[tokio::test]
    async fn eviction_on_time_test() {
        let cache = Cache::new("test", 2, 2, None);
        cache
            .push(
                CacheKey {
                    key: "key".to_string(),
                    prefix: "prefix".to_string(),
                },
                "val".to_string(),
            )
            .await;
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        assert_eq!(
            cache
                .get_val::<String>(CacheKey {
                    key: "key".to_string(),
                    prefix: "prefix".to_string()
                })
                .await,
            None
        );
    }
}
