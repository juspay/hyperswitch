use std::{
    any::Any,
    borrow::Cow,
    fmt::Debug,
    sync::{Arc, LazyLock},
};

use common_utils::{
    errors::{self, CustomResult},
    ext_traits::ByteSliceExt,
};
use dyn_clone::DynClone;
use error_stack::{Report, ResultExt};
use moka::{future::Cache as MokaCache, ops::compute::Op};
use redis_interface::{errors::RedisError, RedisConnectionWithContext, RedisValue};
use router_env::{
    logger,
    tracing::{self, instrument},
};

use crate::{
    errors::StorageError,
    metrics,
    redis::{kv_store::RedisConnInterface, pub_sub::PubSubInterface},
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
pub static CONFIG_CACHE: LazyLock<Cache> =
    LazyLock::new(|| Cache::new("CONFIG_CACHE", CACHE_TTL, CACHE_TTI, None));

/// Accounts cache with time_to_live as 30 mins and size limit
pub static ACCOUNTS_CACHE: LazyLock<Cache> =
    LazyLock::new(|| Cache::new("ACCOUNTS_CACHE", CACHE_TTL, CACHE_TTI, Some(MAX_CAPACITY)));

/// Merchant connector account list cache.
///
/// Holds whole-scope supersets of merchant connector account rows (every account of a
/// merchant, or of a profile) which the individual list queries project from. Kept
/// separate from [`ACCOUNTS_CACHE`] so that list hit rates and entry counts are
/// observable on their own, and so the two can be sized independently.
pub static MCA_LIST_CACHE: LazyLock<Cache> =
    LazyLock::new(|| Cache::new("MCA_LIST_CACHE", CACHE_TTL, CACHE_TTI, Some(MAX_CAPACITY)));

/// Routing Cache
pub static ROUTING_CACHE: LazyLock<Cache> =
    LazyLock::new(|| Cache::new("ROUTING_CACHE", CACHE_TTL, CACHE_TTI, Some(MAX_CAPACITY)));

/// 3DS Decision Manager Cache
pub static DECISION_MANAGER_CACHE: LazyLock<Cache> = LazyLock::new(|| {
    Cache::new(
        "DECISION_MANAGER_CACHE",
        CACHE_TTL,
        CACHE_TTI,
        Some(MAX_CAPACITY),
    )
});

/// Surcharge Cache
pub static SURCHARGE_CACHE: LazyLock<Cache> =
    LazyLock::new(|| Cache::new("SURCHARGE_CACHE", CACHE_TTL, CACHE_TTI, Some(MAX_CAPACITY)));

/// CGraph Cache
pub static CGRAPH_CACHE: LazyLock<Cache> =
    LazyLock::new(|| Cache::new("CGRAPH_CACHE", CACHE_TTL, CACHE_TTI, Some(MAX_CAPACITY)));

/// PM Filter CGraph Cache
pub static PM_FILTERS_CGRAPH_CACHE: LazyLock<Cache> = LazyLock::new(|| {
    Cache::new(
        "PM_FILTERS_CGRAPH_CACHE",
        CACHE_TTL,
        CACHE_TTI,
        Some(MAX_CAPACITY),
    )
});

/// Success based Dynamic Algorithm Cache
pub static SUCCESS_BASED_DYNAMIC_ALGORITHM_CACHE: LazyLock<Cache> = LazyLock::new(|| {
    Cache::new(
        "SUCCESS_BASED_DYNAMIC_ALGORITHM_CACHE",
        CACHE_TTL,
        CACHE_TTI,
        Some(MAX_CAPACITY),
    )
});

/// Elimination based Dynamic Algorithm Cache
pub static ELIMINATION_BASED_DYNAMIC_ALGORITHM_CACHE: LazyLock<Cache> = LazyLock::new(|| {
    Cache::new(
        "ELIMINATION_BASED_DYNAMIC_ALGORITHM_CACHE",
        CACHE_TTL,
        CACHE_TTI,
        Some(MAX_CAPACITY),
    )
});

/// Contract Routing based Dynamic Algorithm Cache
pub static CONTRACT_BASED_DYNAMIC_ALGORITHM_CACHE: LazyLock<Cache> = LazyLock::new(|| {
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
    MerchantConnectorAccountList(Cow<'a, str>),
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
            | CacheKind::MerchantConnectorAccountList(key)
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

/// The physical moka key for a logical [`CacheKey`].
///
/// During replay the in-memory cache is a process-global structure shared across correlations, so
/// its keys are namespaced by correlation id — the same isolation
/// `RedisConnectionPool::add_prefix` applies to physical redis keys. This keeps a replayed
/// correlation from observing entries another one populated: the first lookup per correlation
/// misses and falls through to the redis and database boundaries, which are instrumented and
/// therefore deterministic.
///
/// Deliberately not folded into `From<CacheKey> for String`: that conversion also builds the
/// recorded args for the `in_memory_get` boundary, which must stay un-namespaced so the args a
/// replay computes still match the ones the recording captured.
fn in_memory_cache_key(key: CacheKey) -> String {
    let physical = String::from(key);

    #[cfg(feature = "deja")]
    if let Some(correlation_id) = deja::replay_key_namespace() {
        return format!("{correlation_id}:{physical}");
    }

    physical
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

    // Deja: recorded args-only for population accounting; the real moka insert
    // still runs on replay (`replay = Execute`).
    #[cfg_attr(
        feature = "deja",
        deja::boundary(
            boundary = "imc",
            component = "storage_impl::redis::cache",
            operation = "in_memory_push",
            replay = Execute,
            effect = Imc,
            codec = SerdeCodec,
            args = deja_in_memory_args(self.name, &key),
        )
    )]
    pub async fn push<T: Cacheable>(&self, key: CacheKey, val: T) {
        self.inner
            .insert(in_memory_cache_key(key), Arc::new(val))
            .await;
    }

    // Deja: the L1 seam, instrumented on the method itself so no call path can
    // bypass it. A recorded `Some(v)` substitutes on replay; a recorded `None`
    // re-triggers the caller's fallback. The serde bound is deliberately
    // unconditional: a type that cannot be captured cannot be cached.
    #[cfg_attr(
        feature = "deja",
        deja::boundary(
            boundary = "imc",
            component = "storage_impl::redis::cache",
            operation = "in_memory_get",
            replay = Substitute,
            effect = Imc,
            codec = SerdeCodec,
            args = deja_in_memory_args(self.name, &key),
        )
    )]
    pub async fn get_val<T>(&self, key: CacheKey) -> Option<T>
    where
        T: Clone + Cacheable + serde::Serialize + serde::de::DeserializeOwned,
    {
        let val = self.inner.get::<String>(&in_memory_cache_key(key)).await;

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

    /// Returns a cached value, loading it on a miss.
    ///
    /// Concurrent callers for the same key share one load per process.
    /// Other instances may still load the same key concurrently.
    async fn get_val_or_load<T, F, Fut>(
        &self,
        key: CacheKey,
        fun: F,
    ) -> CustomResult<T, StorageError>
    where
        T: Cacheable + Clone,
        F: FnOnce() -> Fut + Send,
        Fut: futures::Future<Output = CustomResult<T, StorageError>> + Send,
    {
        let cache_name = self.name;

        self.inner
            .entry(in_memory_cache_key(key))
            .and_try_compute_with(|entry| async move {
                // A value of another type counts as a miss, as `get_val` treats it.
                let is_cached = entry
                    .as_ref()
                    .is_some_and(|entry| (**entry.value()).as_any().is::<T>());

                let op = if is_cached {
                    Op::Nop
                } else {
                    let value: Arc<dyn Cacheable> = Arc::new(fun().await?);
                    Op::Put(value)
                };
                Ok::<_, Report<StorageError>>(op)
            })
            .await?
            .into_entry()
            .and_then(|entry| {
                let value = entry.into_value();
                (*value).as_any().downcast_ref::<T>().cloned()
            })
            .ok_or_else(|| Report::new(StorageError::DeserializationFailed))
            .attach_printable_lazy(|| format!("Unexpected cached value type in {cache_name}"))
    }

    /// Check if a key exists in cache
    #[cfg_attr(
        feature = "deja",
        deja::boundary(
            boundary = "imc",
            component = "storage_impl::redis::cache",
            operation = "in_memory_exists",
            replay = Substitute,
            effect = Imc,
            codec = SerdeCodec,
            args = deja_in_memory_args(self.name, &key),
        )
    )]
    pub async fn exists(&self, key: CacheKey) -> bool {
        self.inner.contains_key::<String>(&in_memory_cache_key(key))
    }

    // Deja: recorded args-only so a missed or extra invalidation shows up as a
    // call divergence; the real invalidation still runs on replay.
    #[cfg_attr(
        feature = "deja",
        deja::boundary(
            boundary = "imc",
            component = "storage_impl::redis::cache",
            operation = "in_memory_remove",
            replay = Execute,
            effect = Imc,
            codec = SerdeCodec,
            args = deja_in_memory_args(self.name, &key),
        )
    )]
    pub async fn remove(&self, key: CacheKey) {
        self.inner
            .invalidate::<String>(&in_memory_cache_key(key))
            .await;
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
    redis: &RedisConnectionWithContext,
    key: impl AsRef<str>,
    ttl: Option<i64>,
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
        match ttl {
            Some(ttl) => {
                redis
                    .serialize_and_set_key_with_expiry(&key.into(), &data, ttl)
                    .await
            }
            None => redis.serialize_and_set_key(&key.into(), &data).await,
        }
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

/// Recorded args for the [`Cache`] boundaries: cache name + un-namespaced
/// physical key, so the args a replay computes match the recording's.
#[cfg(feature = "deja")]
fn deja_in_memory_args(cache_name: &str, key: &CacheKey) -> serde_json::Value {
    serde_json::json!({ "cache": cache_name, "key": String::from(key.clone()) })
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
    let cache_key = CacheKey {
        key: key.to_string(),
        prefix: redis.redis_conn.key_prefix.clone(),
    };
    let cache_val = cache.get_val::<T>(cache_key).await;
    if let Some(val) = cache_val {
        Ok(val)
    } else {
        cache
            .get_val_or_load(
                CacheKey {
                    key: key.to_string(),
                    prefix: redis.redis_conn.key_prefix.clone(),
                },
                || get_or_populate_redis(redis, key, None, fun),
            )
            .await
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

    let redis_conn = &redis_conn;
    let futures = keys.into_iter().map(move |key| async move {
        redis_conn
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
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    use redis_interface::{RedisConnectionPool, RedisConnectionWithContext, RedisSettings};
    use tokio::sync::Barrier;

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

    /// Minimal [`RedisConnInterface`] over a real Redis, as `redis_interface` tests connect.
    struct TestStore {
        pool: Arc<RedisConnectionPool>,
    }

    impl TestStore {
        async fn new() -> Self {
            let pool = RedisConnectionPool::new_without_event_emitter(&RedisSettings::default())
                .await
                .expect("failed to connect to Redis");

            Self {
                pool: Arc::new(pool),
            }
        }

        async fn delete(&self, key: &str) {
            self.get_redis_conn()
                .expect("redis connection")
                .delete_key(&key.into())
                .await
                .expect("redis key deletion");
        }
    }

    impl RedisConnInterface for TestStore {
        fn get_redis_conn(&self) -> error_stack::Result<RedisConnectionWithContext, RedisError> {
            Ok(RedisConnectionWithContext::new_without_context(Arc::clone(
                &self.pool,
            )))
        }
    }

    static KEY_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

    /// A key that is absent from Redis, so the fallback is the only way to a value.
    fn unique_key(tag: &str) -> String {
        format!(
            "cache_test_{tag}_{}_{}_{}",
            common_utils::process_id(),
            common_utils::date_time::now_unix_timestamp_millis(),
            KEY_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        )
    }

    fn no_prefix_key(key: &str) -> CacheKey {
        CacheKey {
            key: key.to_string(),
            prefix: String::new(),
        }
    }

    #[tokio::test]
    async fn concurrent_misses_for_same_key_load_once() {
        const CONCURRENT_REQUESTS: usize = 100;

        let store = Arc::new(TestStore::new().await);
        let cache = Arc::new(Cache::new("single_flight_test", 1800, 1800, None));
        let key = unique_key("same_key");

        let fallback_count = Arc::new(AtomicUsize::new(0));
        let start = Arc::new(Barrier::new(CONCURRENT_REQUESTS));

        let mut handles = Vec::with_capacity(CONCURRENT_REQUESTS);

        for _ in 0..CONCURRENT_REQUESTS {
            let store = Arc::clone(&store);
            let cache = Arc::clone(&cache);
            let key = key.clone();
            let fallback_count = Arc::clone(&fallback_count);
            let start = Arc::clone(&start);

            handles.push(tokio::spawn(async move {
                start.wait().await;

                get_or_populate_in_memory(
                    store.as_ref(),
                    &key,
                    || {
                        let fallback_count = Arc::clone(&fallback_count);

                        async move {
                            fallback_count.fetch_add(1, Ordering::SeqCst);
                            // Wide enough that every caller misses before the first one returns.
                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                            Ok("loaded_from_database".to_string())
                        }
                    },
                    &cache,
                )
                .await
                .expect("cache load should succeed")
            }));
        }

        for handle in handles {
            assert_eq!(
                handle.await.expect("task should complete"),
                "loaded_from_database"
            );
        }

        let executions = fallback_count.load(Ordering::SeqCst);
        println!("Concurrent requests: {CONCURRENT_REQUESTS}, fallback executions: {executions}");
        assert_eq!(executions, 1);

        store.delete(&key).await;
    }

    #[tokio::test]
    async fn concurrent_misses_for_distinct_keys_are_not_serialized() {
        const CONCURRENT_REQUESTS: usize = 16;

        let store = Arc::new(TestStore::new().await);
        let cache = Arc::new(Cache::new("distinct_keys_test", 1800, 1800, None));
        let key_prefix = unique_key("distinct");

        let fallback_count = Arc::new(AtomicUsize::new(0));
        // Every fallback must be in flight at once, so serialized keys would deadlock here.
        let inside_fallback = Arc::new(Barrier::new(CONCURRENT_REQUESTS));

        let mut handles = Vec::with_capacity(CONCURRENT_REQUESTS);

        for index in 0..CONCURRENT_REQUESTS {
            let store = Arc::clone(&store);
            let cache = Arc::clone(&cache);
            let key = format!("{key_prefix}_{index}");
            let fallback_count = Arc::clone(&fallback_count);
            let inside_fallback = Arc::clone(&inside_fallback);

            handles.push(tokio::spawn(async move {
                let value = get_or_populate_in_memory(
                    store.as_ref(),
                    &key,
                    || {
                        let fallback_count = Arc::clone(&fallback_count);
                        let inside_fallback = Arc::clone(&inside_fallback);
                        let key = key.clone();

                        async move {
                            fallback_count.fetch_add(1, Ordering::SeqCst);
                            inside_fallback.wait().await;
                            Ok(key)
                        }
                    },
                    &cache,
                )
                .await
                .expect("cache load should succeed");

                assert_eq!(value, key);
                key
            }));
        }

        let keys = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            futures::future::join_all(handles),
        )
        .await
        .expect("distinct keys must load concurrently, not one after another");

        assert_eq!(fallback_count.load(Ordering::SeqCst), CONCURRENT_REQUESTS);

        for key in keys {
            store.delete(&key.expect("task should complete")).await;
        }
    }

    #[tokio::test]
    async fn fallback_error_does_not_poison_key() {
        const CONCURRENT_REQUESTS: usize = 16;

        let store = Arc::new(TestStore::new().await);
        let cache = Arc::new(Cache::new("error_test", 1800, 1800, None));
        let key = unique_key("failing");

        let attempts = Arc::new(AtomicUsize::new(0));
        let start = Arc::new(Barrier::new(CONCURRENT_REQUESTS));

        let mut handles = Vec::with_capacity(CONCURRENT_REQUESTS);

        for _ in 0..CONCURRENT_REQUESTS {
            let store = Arc::clone(&store);
            let cache = Arc::clone(&cache);
            let key = key.clone();
            let attempts = Arc::clone(&attempts);
            let start = Arc::clone(&start);

            handles.push(tokio::spawn(async move {
                start.wait().await;

                get_or_populate_in_memory::<String, _, _>(
                    store.as_ref(),
                    &key,
                    || {
                        let attempts = Arc::clone(&attempts);

                        async move {
                            attempts.fetch_add(1, Ordering::SeqCst);
                            Err(Report::new(StorageError::ValueNotFound("gone".to_string())))
                        }
                    },
                    &cache,
                )
                .await
            }));
        }

        for handle in handles {
            let error = handle
                .await
                .expect("task should complete")
                .expect_err("failing fallback must surface its error");

            assert!(matches!(
                error.current_context(),
                StorageError::ValueNotFound(_)
            ));
        }

        assert!(attempts.load(Ordering::SeqCst) >= 1);
        assert!(!cache.exists(no_prefix_key(&key)).await);

        let recovered = get_or_populate_in_memory(
            store.as_ref(),
            &key,
            || async { Ok("recovered".to_string()) },
            &cache,
        )
        .await
        .expect("retry after a failed fallback should succeed");

        assert_eq!(recovered, "recovered");
        assert_eq!(
            cache.get_val::<String>(no_prefix_key(&key)).await,
            Some("recovered".to_string())
        );

        store.delete(&key).await;
    }

    /// Single-flight is process-local; other router instances may still load the same key.
    #[tokio::test]
    async fn concurrent_redis_misses_are_not_coalesced() {
        const CONCURRENT_REQUESTS: usize = 100;

        let store = Arc::new(TestStore::new().await);
        let redis = Arc::new(store.get_redis_conn().expect("redis connection"));
        let key = unique_key("redis_only");

        let fallback_count = Arc::new(AtomicUsize::new(0));
        let start = Arc::new(Barrier::new(CONCURRENT_REQUESTS));

        let mut handles = Vec::with_capacity(CONCURRENT_REQUESTS);

        for _ in 0..CONCURRENT_REQUESTS {
            let redis = Arc::clone(&redis);
            let key = key.clone();
            let fallback_count = Arc::clone(&fallback_count);
            let start = Arc::clone(&start);

            handles.push(tokio::spawn(async move {
                start.wait().await;

                get_or_populate_redis(redis.as_ref(), &key, None, || {
                    let fallback_count = Arc::clone(&fallback_count);

                    async move {
                        fallback_count.fetch_add(1, Ordering::SeqCst);
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        Ok("loaded_from_database".to_string())
                    }
                })
                .await
                .expect("get_or_populate_redis should succeed")
            }));
        }

        for handle in handles {
            assert_eq!(
                handle.await.expect("task should complete"),
                "loaded_from_database"
            );
        }

        let executions = fallback_count.load(Ordering::SeqCst);
        println!("Concurrent Redis requests: {CONCURRENT_REQUESTS}");
        println!("Fallback executions: {executions}");
        assert!(executions >= 1);

        store.delete(&key).await;
    }
}
