use std::{any::Any, borrow::Cow, fmt::Debug, sync::Arc};

use common_utils::{
    errors::{self, CustomResult},
    ext_traits::ByteSliceExt,
};
use dyn_clone::DynClone;
use error_stack::{Report, ResultExt};
use moka::future::Cache as MokaCache;
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

/// Default redis channel name used for publishing invalidation messages
pub const DEFAULT_IMC_INVALIDATION_CHANNEL: &str = "hyperswitch_invalidate";

/// Default time to live 30 mins
pub const DEFAULT_CACHE_TTL: u64 = 30 * 60;

/// Default time to idle 10 mins
pub const DEFAULT_CACHE_TTI: u64 = 10 * 60;

/// Default max capacity of cache in MB
pub const DEFAULT_MAX_CAPACITY: u64 = 30;

/// Runtime overrides for a single in-memory cache.
///
/// Every field is optional: whatever is left unset falls back to that cache's compiled-in
/// default, so an absent (or partial) configuration reproduces the values the caches were
/// previously hardcoded with.
#[derive(Debug, Clone, Copy, Default, serde::Deserialize)]
#[serde(default)]
pub struct CacheSettings {
    /// Time in seconds an entry is retained after it was inserted
    pub ttl_in_secs: Option<u64>,
    /// Time in seconds an entry is retained after it was last read or written
    pub tti_in_secs: Option<u64>,
    /// Max size in MB the cache may hold. `0` makes the cache unbounded.
    pub max_capacity_in_mb: Option<u64>,
}

impl CacheSettings {
    fn time_to_live(&self) -> u64 {
        self.ttl_in_secs.unwrap_or(DEFAULT_CACHE_TTL)
    }

    fn time_to_idle(&self) -> u64 {
        self.tti_in_secs.unwrap_or(DEFAULT_CACHE_TTI)
    }

    /// Resolves the max capacity against the cache's own default, `None` meaning unbounded.
    ///
    /// An explicitly configured `0` is how a bounded cache is made unbounded, since the
    /// absence of the key already means "use the default".
    fn max_capacity(&self, default: Option<u64>) -> Option<u64> {
        match self.max_capacity_in_mb {
            Some(0) => None,
            Some(capacity) => Some(capacity),
            None => default,
        }
    }

    /// Builds the cache this configuration describes.
    ///
    /// `default_max_capacity` is the cache's own capacity default, `None` meaning unbounded.
    fn build(&self, name: &'static str, default_max_capacity: Option<u64>) -> Cache {
        Cache::new(
            name,
            self.time_to_live(),
            self.time_to_idle(),
            self.max_capacity(default_max_capacity),
        )
    }
}

/// Runtime configuration of the in-memory caches, with per-cache granularity.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct CacheConfig {
    /// Redis channel the invalidation messages are published on and subscribed to.
    /// All instances of a deployment must agree on this value.
    pub invalidation_channel: Option<String>,
    pub config: CacheSettings,
    pub accounts: CacheSettings,
    pub mca_list: CacheSettings,
    pub routing: CacheSettings,
    pub decision_manager: CacheSettings,
    pub surcharge: CacheSettings,
    pub cgraph: CacheSettings,
    pub pm_filters_cgraph: CacheSettings,
    pub success_based_dynamic_algorithm: CacheSettings,
    pub elimination_based_dynamic_algorithm: CacheSettings,
    pub contract_based_dynamic_algorithm: CacheSettings,
}

/// The in-memory caches, together with the redis channel their invalidations travel on.
///
/// Built once per process and shared by every tenant's store through an [`Arc`]: entries are
/// isolated by the tenant prefix carried in [`CacheKey`], and the single redis subscriber that
/// applies invalidations has to reach the very same instances the stores read from. Handing a
/// tenant its own set would both multiply the memory budget and strand every tenant but one
/// with stale entries.
#[derive(Debug)]
pub struct Caches {
    /// Redis channel invalidation messages are published on and subscribed to
    pub invalidation_channel: String,
    /// Config cache, unbounded by default
    pub config: Cache,
    /// Accounts cache
    pub accounts: Cache,
    /// Merchant connector account list cache.
    ///
    /// Holds whole-scope supersets of merchant connector account rows (every account of a
    /// merchant, or of a profile) which the individual list queries project from. Kept
    /// separate from [`Caches::accounts`] so that list hit rates and entry counts are
    /// observable on their own, and so the two can be sized independently.
    pub mca_list: Cache,
    /// Routing cache
    pub routing: Cache,
    /// 3DS decision manager cache
    pub decision_manager: Cache,
    /// Surcharge cache
    pub surcharge: Cache,
    /// CGraph cache
    pub cgraph: Cache,
    /// PM filter CGraph cache
    pub pm_filters_cgraph: Cache,
    /// Success based dynamic algorithm cache
    pub success_based_dynamic_algorithm: Cache,
    /// Elimination based dynamic algorithm cache
    pub elimination_based_dynamic_algorithm: Cache,
    /// Contract routing based dynamic algorithm cache
    pub contract_based_dynamic_algorithm: Cache,
}

impl Caches {
    /// Builds every cache from `config`, falling back to the per-cache defaults for anything
    /// left unset.
    pub fn new(config: &CacheConfig) -> Self {
        let bounded = Some(DEFAULT_MAX_CAPACITY);

        Self {
            invalidation_channel: config
                .invalidation_channel
                .clone()
                .unwrap_or_else(|| DEFAULT_IMC_INVALIDATION_CHANNEL.to_string()),
            config: config.config.build("CONFIG_CACHE", None),
            accounts: config.accounts.build("ACCOUNTS_CACHE", bounded),
            mca_list: config.mca_list.build("MCA_LIST_CACHE", bounded),
            routing: config.routing.build("ROUTING_CACHE", bounded),
            decision_manager: config
                .decision_manager
                .build("DECISION_MANAGER_CACHE", bounded),
            surcharge: config.surcharge.build("SURCHARGE_CACHE", bounded),
            cgraph: config.cgraph.build("CGRAPH_CACHE", bounded),
            pm_filters_cgraph: config
                .pm_filters_cgraph
                .build("PM_FILTERS_CGRAPH_CACHE", bounded),
            success_based_dynamic_algorithm: config
                .success_based_dynamic_algorithm
                .build("SUCCESS_BASED_DYNAMIC_ALGORITHM_CACHE", bounded),
            elimination_based_dynamic_algorithm: config
                .elimination_based_dynamic_algorithm
                .build("ELIMINATION_BASED_DYNAMIC_ALGORITHM_CACHE", bounded),
            contract_based_dynamic_algorithm: config
                .contract_based_dynamic_algorithm
                .build("CONTRACT_BASED_DYNAMIC_ALGORITHM_CACHE", bounded),
        }
    }

    /// The caches an invalidation of `kind` has to be applied to.
    pub fn for_kind(&self, kind: &CacheKind<'_>) -> Vec<&Cache> {
        match kind {
            CacheKind::Config(_) => vec![&self.config],
            CacheKind::Accounts(_) => vec![&self.accounts],
            CacheKind::MerchantConnectorAccountList(_) => vec![&self.mca_list],
            CacheKind::Routing(_) => vec![&self.routing],
            CacheKind::DecisionManager(_) => vec![&self.decision_manager],
            CacheKind::Surcharge(_) => vec![&self.surcharge],
            CacheKind::CGraph(_) => vec![&self.cgraph],
            CacheKind::PmFiltersCGraph(_) => vec![&self.pm_filters_cgraph],
            CacheKind::SuccessBasedDynamicRoutingCache(_) => {
                vec![&self.success_based_dynamic_algorithm]
            }
            CacheKind::EliminationBasedDynamicRoutingCache(_) => {
                vec![&self.elimination_based_dynamic_algorithm]
            }
            CacheKind::ContractBasedDynamicRoutingCache(_) => {
                vec![&self.contract_based_dynamic_algorithm]
            }
            CacheKind::All(_) => self.all().to_vec(),
        }
    }

    /// The cache `id` names.
    pub fn get(&self, id: CacheId) -> &Cache {
        match id {
            CacheId::Config => &self.config,
            CacheId::Accounts => &self.accounts,
            CacheId::McaList => &self.mca_list,
            CacheId::Routing => &self.routing,
            CacheId::DecisionManager => &self.decision_manager,
            CacheId::Surcharge => &self.surcharge,
            CacheId::CGraph => &self.cgraph,
            CacheId::PmFiltersCGraph => &self.pm_filters_cgraph,
            CacheId::SuccessBasedDynamicAlgorithm => &self.success_based_dynamic_algorithm,
            CacheId::EliminationBasedDynamicAlgorithm => &self.elimination_based_dynamic_algorithm,
            CacheId::ContractBasedDynamicAlgorithm => &self.contract_based_dynamic_algorithm,
        }
    }

    /// Every cache in the set, so that callers iterating over all of them — metrics
    /// collection, say — cannot silently fall behind a newly added cache.
    pub fn all(&self) -> [&Cache; 11] {
        [
            &self.config,
            &self.accounts,
            &self.mca_list,
            &self.routing,
            &self.decision_manager,
            &self.surcharge,
            &self.cgraph,
            &self.pm_filters_cgraph,
            &self.success_based_dynamic_algorithm,
            &self.elimination_based_dynamic_algorithm,
            &self.contract_based_dynamic_algorithm,
        ]
    }
}

impl Default for Caches {
    fn default() -> Self {
        Self::new(&CacheConfig::default())
    }
}

/// A store that owns a set of in-memory caches, the way [`RedisConnInterface`] gives one a
/// redis connection. Deliberately independent of it: holding caches and holding a redis
/// connection are separate capabilities, and the helpers below ask for both where they need
/// both.
pub trait CacheInterface {
    fn caches(&self) -> &Caches;

    /// The tenant prefix in-memory cache keys are namespaced by.
    ///
    /// Available without a redis connection, so an in-memory hit never has to acquire one.
    fn cache_key_prefix(&self) -> &str;
}

/// Names one of the [`Caches`].
///
/// Lookups name the cache they want rather than passing an instance, so the cache they read
/// is necessarily the one belonging to the store they are given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheId {
    Config,
    Accounts,
    McaList,
    Routing,
    DecisionManager,
    Surcharge,
    CGraph,
    PmFiltersCGraph,
    SuccessBasedDynamicAlgorithm,
    EliminationBasedDynamicAlgorithm,
    ContractBasedDynamicAlgorithm,
}
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

impl Debug for Cache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cache")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
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
pub async fn get_or_populate_in_memory<T, F, Fut, S>(
    store: &S,
    key: &str,
    fun: F,
    cache: CacheId,
) -> CustomResult<T, StorageError>
where
    T: Cacheable + serde::Serialize + serde::de::DeserializeOwned + Debug + Clone,
    F: FnOnce() -> Fut + Send,
    Fut: futures::Future<Output = CustomResult<T, StorageError>> + Send,
    S: RedisConnInterface + CacheInterface + Send + Sync + ?Sized,
{
    let cache = store.caches().get(cache);
    let cache_key = CacheKey {
        key: key.to_string(),
        prefix: store.cache_key_prefix().to_string(),
    };

    // An in-memory hit answers on its own. The redis connection is acquired only on a miss,
    // so a redis outage degrades this to a cold cache rather than an error.
    if let Some(val) = cache.get_val::<T>(cache_key.clone()).await {
        return Ok(val);
    }

    let redis = &store
        .get_redis_conn()
        .change_context(StorageError::RedisError(
            RedisError::RedisConnectionError.into(),
        ))
        .attach_printable("Failed to get redis connection")?;
    let val = get_or_populate_redis(redis, key, None, fun).await?;
    cache.push(cache_key, val.clone()).await;

    Ok(val)
}

#[instrument(skip_all)]
pub async fn redact_from_redis_and_publish<'a, K, S>(
    store: &S,
    keys: K,
) -> CustomResult<usize, StorageError>
where
    K: IntoIterator<Item = CacheKind<'a>> + Send + Clone,
    S: RedisConnInterface + CacheInterface + Send + Sync + ?Sized,
{
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
            .publish(&store.caches().invalidation_channel, key)
            .await
            .change_context(StorageError::KVError)
    });

    Ok(futures::future::try_join_all(futures)
        .await?
        .iter()
        .sum::<usize>())
}

#[instrument(skip_all)]
pub async fn publish_and_redact<'a, T, F, Fut, S>(
    store: &S,
    key: CacheKind<'a>,
    fun: F,
) -> CustomResult<T, StorageError>
where
    F: FnOnce() -> Fut + Send,
    Fut: futures::Future<Output = CustomResult<T, StorageError>> + Send,
    S: RedisConnInterface + CacheInterface + Send + Sync + ?Sized,
{
    let data = fun().await?;
    redact_from_redis_and_publish(store, [key]).await?;
    Ok(data)
}

#[instrument(skip_all)]
pub async fn publish_and_redact_multiple<'a, T, F, Fut, K, S>(
    store: &S,
    keys: K,
    fun: F,
) -> CustomResult<T, StorageError>
where
    F: FnOnce() -> Fut + Send,
    Fut: futures::Future<Output = CustomResult<T, StorageError>> + Send,
    K: IntoIterator<Item = CacheKind<'a>> + Send + Clone,
    S: RedisConnInterface + CacheInterface + Send + Sync + ?Sized,
{
    let data = fun().await?;
    redact_from_redis_and_publish(store, keys).await?;
    Ok(data)
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    #[test]
    fn unset_settings_resolve_to_the_compiled_in_defaults() {
        let settings = CacheSettings::default();

        assert_eq!(settings.time_to_live(), DEFAULT_CACHE_TTL);
        assert_eq!(settings.time_to_idle(), DEFAULT_CACHE_TTI);
        // Each cache keeps its own capacity default: unbounded for `CONFIG_CACHE`, 30 MB
        // for the rest.
        assert_eq!(settings.max_capacity(None), None);
        assert_eq!(
            settings.max_capacity(Some(DEFAULT_MAX_CAPACITY)),
            Some(DEFAULT_MAX_CAPACITY)
        );
    }

    #[test]
    fn configured_settings_override_the_defaults() {
        let settings = CacheSettings {
            ttl_in_secs: Some(60),
            tti_in_secs: Some(30),
            max_capacity_in_mb: Some(128),
        };

        assert_eq!(settings.time_to_live(), 60);
        assert_eq!(settings.time_to_idle(), 30);
        assert_eq!(settings.max_capacity(None), Some(128));
        assert_eq!(settings.max_capacity(Some(DEFAULT_MAX_CAPACITY)), Some(128));
    }

    #[test]
    fn a_zero_max_capacity_makes_a_bounded_cache_unbounded() {
        let settings = CacheSettings {
            max_capacity_in_mb: Some(0),
            ..CacheSettings::default()
        };

        assert_eq!(settings.max_capacity(Some(DEFAULT_MAX_CAPACITY)), None);
    }

    #[test]
    fn partially_configured_caches_deserialize_with_defaults_for_the_rest() {
        let config: CacheConfig = serde_json::from_value(serde_json::json!({
            "accounts": { "ttl_in_secs": 120 },
        }))
        .expect("failed to deserialize cache configuration");

        assert_eq!(config.accounts.time_to_live(), 120);
        assert_eq!(config.accounts.time_to_idle(), DEFAULT_CACHE_TTI);
        assert_eq!(
            config.accounts.max_capacity(Some(DEFAULT_MAX_CAPACITY)),
            Some(DEFAULT_MAX_CAPACITY)
        );
        assert_eq!(config.routing.time_to_live(), DEFAULT_CACHE_TTL);
        assert_eq!(config.config.max_capacity(None), None);
    }

    #[test]
    fn an_unset_invalidation_channel_falls_back_to_the_default() {
        let caches = Caches::default();

        assert_eq!(caches.invalidation_channel, "hyperswitch_invalidate");
    }

    #[test]
    fn a_cache_id_resolves_to_the_cache_it_names() {
        let caches = Caches::default();

        assert_eq!(caches.get(CacheId::Config).name(), "CONFIG_CACHE");
        assert_eq!(caches.get(CacheId::Accounts).name(), "ACCOUNTS_CACHE");
        assert_eq!(caches.get(CacheId::McaList).name(), "MCA_LIST_CACHE");
        assert_eq!(
            caches.get(CacheId::ContractBasedDynamicAlgorithm).name(),
            "CONTRACT_BASED_DYNAMIC_ALGORITHM_CACHE"
        );
    }

    #[test]
    fn every_cache_is_reachable_from_all() {
        let caches = Caches::default();
        let names = caches.all().map(Cache::name);

        assert_eq!(names.len(), 11);
        assert!(names.contains(&"CONFIG_CACHE"));
        assert!(names.contains(&"CONTRACT_BASED_DYNAMIC_ALGORITHM_CACHE"));
    }

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
