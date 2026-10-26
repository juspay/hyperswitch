use std::{
    any::Any,
    borrow::Cow,
    collections::HashMap,
    fmt::Debug,
    num::NonZeroU64,
    sync::{Arc, Mutex},
    time::Duration,
};

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

/// Ceiling on how long a single populate attempt may run, 500ms — well above the worst-case
/// chain of a redis read, a DB read, and a redis write (sub-millisecond, 3-5ms,
/// sub-millisecond respectively). Not deployment-configurable: no `config/*.toml` has ever
/// set the field this used to back.
pub const DEFAULT_POPULATE_TIMEOUT_IN_MS: NonZeroU64 =
    NonZeroU64::new(500).expect("500 is nonzero");

/// Largest percentage of a redis TTL that jitter may add on top: `10` -> up to 10% more.
const REDIS_TTL_JITTER_MAX_PERCENT: u8 = 10;

/// How many times its in-memory lifetime an entry is kept in redis.
///
/// Redis sits behind moka, so it spares a database read only while it outlives it. A
/// multiple of 1 would lapse moments after the moka entry it refills and sit empty until
/// the next miss; doubling keeps it populated across the miss that matters.
const REDIS_TTL_MULTIPLE: u8 = 2;

/// Default entry ceiling, deliberately the number the caches have always been built with.
///
/// It was written as `30` and documented as megabytes, then multiplied by `1024 * 1024` on the
/// way into moka — which, with no weigher configured, read it as a number of entries. So the
/// effective ceiling has always been 31,457,280 entries, and these caches have never come
/// close to it.
///
/// Kept verbatim rather than replaced with a tighter or byte-based default: at this size it
/// binds nothing, which is exactly the point — the ceiling that ships must not start evicting
/// where nothing evicted before. What bounds the resident set in normal operation is
/// `time_to_idle`, since these caches are keyed per merchant and profile and an entry unread
/// for its window is dropped.
///
/// Choosing a ceiling that actually binds is an operational decision rather than a default:
/// set `max_capacity` per cache, in either unit, once the sizes involved are known.
pub const DEFAULT_MAX_ENTRIES: u64 = 30 * 1024 * 1024;

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
    /// Maximum number of entries the cache may hold. `0` makes it unbounded, and leaving it
    /// unset keeps the cache's own default.
    pub max_entries: Option<u64>,
}

impl CacheSettings {
    fn time_to_live(&self) -> u64 {
        self.ttl_in_secs.unwrap_or(DEFAULT_CACHE_TTL)
    }

    fn time_to_idle(&self) -> u64 {
        self.tti_in_secs.unwrap_or(DEFAULT_CACHE_TTI)
    }

    /// Resolves the ceiling this configuration asks for.
    ///
    /// `None` means unbounded, and an explicitly configured `0` is how a bounded cache is made
    /// unbounded, since the key's absence already means "use the default".
    fn max_entries(&self, default: Option<u64>) -> Option<u64> {
        match self.max_entries {
            Some(0) => None,
            Some(entries) => Some(entries),
            None => default,
        }
    }

    /// Builds the cache this configuration describes.
    ///
    /// `id` names the cache being built, and supplies the name it reports itself under in
    /// metrics. `default` is the cache's own ceiling when the configuration names none,
    /// `None` meaning unbounded.
    fn build(&self, id: CacheId, default: Option<u64>) -> Cache {
        Cache::new(
            id.name(),
            self.time_to_live(),
            self.time_to_idle(),
            self.max_entries(default),
            Duration::from_millis(DEFAULT_POPULATE_TIMEOUT_IN_MS.get()),
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
        // Every cache defaults to the ceiling it has always had, so shipping this changes no
        // eviction behaviour anywhere.
        const DEFAULT: Option<u64> = Some(DEFAULT_MAX_ENTRIES);
        // As before, the config cache is the one built with no ceiling at all.
        const UNBOUNDED: Option<u64> = None;

        Self {
            invalidation_channel: config
                .invalidation_channel
                .clone()
                .unwrap_or_else(|| DEFAULT_IMC_INVALIDATION_CHANNEL.to_string()),
            config: config.config.build(CacheId::Config, UNBOUNDED),
            accounts: config.accounts.build(CacheId::Accounts, DEFAULT),
            mca_list: config.mca_list.build(CacheId::McaList, DEFAULT),
            routing: config.routing.build(CacheId::Routing, DEFAULT),
            decision_manager: config
                .decision_manager
                .build(CacheId::DecisionManager, DEFAULT),
            surcharge: config.surcharge.build(CacheId::Surcharge, DEFAULT),
            cgraph: config.cgraph.build(CacheId::CGraph, DEFAULT),
            pm_filters_cgraph: config
                .pm_filters_cgraph
                .build(CacheId::PmFiltersCGraph, DEFAULT),
            success_based_dynamic_algorithm: config
                .success_based_dynamic_algorithm
                .build(CacheId::SuccessBasedDynamicAlgorithm, DEFAULT),
            elimination_based_dynamic_algorithm: config
                .elimination_based_dynamic_algorithm
                .build(CacheId::EliminationBasedDynamicAlgorithm, DEFAULT),
            contract_based_dynamic_algorithm: config
                .contract_based_dynamic_algorithm
                .build(CacheId::ContractBasedDynamicAlgorithm, DEFAULT),
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
        let Self {
            config,
            accounts,
            mca_list,
            routing,
            decision_manager,
            surcharge,
            cgraph,
            pm_filters_cgraph,
            success_based_dynamic_algorithm,
            elimination_based_dynamic_algorithm,
            contract_based_dynamic_algorithm,
            invalidation_channel: _,
        } = self;
        match id {
            CacheId::Config => config,
            CacheId::Accounts => accounts,
            CacheId::McaList => mca_list,
            CacheId::Routing => routing,
            CacheId::DecisionManager => decision_manager,
            CacheId::Surcharge => surcharge,
            CacheId::CGraph => cgraph,
            CacheId::PmFiltersCGraph => pm_filters_cgraph,
            CacheId::SuccessBasedDynamicAlgorithm => success_based_dynamic_algorithm,
            CacheId::EliminationBasedDynamicAlgorithm => elimination_based_dynamic_algorithm,
            CacheId::ContractBasedDynamicAlgorithm => contract_based_dynamic_algorithm,
        }
    }

    /// Every cache in the set, so that callers iterating over all of them — metrics
    /// collection, say — cannot silently fall behind a newly added cache.
    pub fn all(&self) -> [&Cache; 11] {
        let Self {
            config,
            accounts,
            mca_list,
            routing,
            decision_manager,
            surcharge,
            cgraph,
            pm_filters_cgraph,
            success_based_dynamic_algorithm,
            elimination_based_dynamic_algorithm,
            contract_based_dynamic_algorithm,
            invalidation_channel: _,
        } = self;
        [
            config,
            accounts,
            mca_list,
            routing,
            decision_manager,
            surcharge,
            cgraph,
            pm_filters_cgraph,
            success_based_dynamic_algorithm,
            elimination_based_dynamic_algorithm,
            contract_based_dynamic_algorithm,
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

impl CacheId {
    /// The name this cache reports itself under in metrics and traces, as the `cache_type`
    /// attribute.
    ///
    /// Kept verbatim: dashboards and alerts match on these strings, so they are part of the
    /// deployment's interface rather than an internal label.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Config => "CONFIG_CACHE",
            Self::Accounts => "ACCOUNTS_CACHE",
            Self::McaList => "MCA_LIST_CACHE",
            Self::Routing => "ROUTING_CACHE",
            Self::DecisionManager => "DECISION_MANAGER_CACHE",
            Self::Surcharge => "SURCHARGE_CACHE",
            Self::CGraph => "CGRAPH_CACHE",
            Self::PmFiltersCGraph => "PM_FILTERS_CGRAPH_CACHE",
            Self::SuccessBasedDynamicAlgorithm => "SUCCESS_BASED_DYNAMIC_ALGORITHM_CACHE",
            Self::EliminationBasedDynamicAlgorithm => "ELIMINATION_BASED_DYNAMIC_ALGORITHM_CACHE",
            Self::ContractBasedDynamicAlgorithm => "CONTRACT_BASED_DYNAMIC_ALGORITHM_CACHE",
        }
    }
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
    /// How long a single populate attempt may run before it's treated as failed. See
    /// [`Self::get_or_populate_in_memory`].
    populate_timeout: Duration,
    loads: Mutex<HashMap<Arc<str>, LoadEntry>>,
}

struct CachePopulation<T> {
    value: T,
    cacheable: bool,
}

struct LoadEntry {
    state: Arc<LoadState>,
    users: usize,
}

struct LoadState {
    key: Arc<str>,
    serial: tokio::sync::Mutex<()>,
    // A load may insert only while this is true. Hold the lock through the insert.
    can_publish: tokio::sync::Mutex<bool>,
}

struct CacheLoad<'a> {
    cache: &'a Cache,
    state: Arc<LoadState>,
}

impl<'a> CacheLoad<'a> {
    fn new(cache: &'a Cache, key: &str) -> Self {
        let mut loads = cache.loads.lock().unwrap_or_else(|err| err.into_inner());
        if let Some(entry) = loads.get_mut(key) {
            entry.users += 1;
            return Self {
                cache,
                state: entry.state.clone(),
            };
        }
        let key: Arc<str> = key.into();
        let state = Arc::new(LoadState {
            key: key.clone(),
            serial: tokio::sync::Mutex::new(()),
            can_publish: tokio::sync::Mutex::new(false),
        });
        loads.insert(
            key,
            LoadEntry {
                state: state.clone(),
                users: 1,
            },
        );
        Self { cache, state }
    }
}

impl Drop for CacheLoad<'_> {
    fn drop(&mut self) {
        let mut loads = self
            .cache
            .loads
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        if let Some(entry) = loads.get_mut(self.state.key.as_ref()) {
            // Count callers under the registry lock. Arc references are dropped
            // after this function returns, so their count cannot decide cleanup.
            entry.users -= 1;
            if entry.users == 0 {
                loads.remove(self.state.key.as_ref());
            }
        }
    }
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

/// Downcasts a stored value back to its concrete type.
///
/// The explicit deref is load-bearing. `Arc<dyn Cacheable>` satisfies the blanket `Cacheable`
/// impl itself, so `val.as_any()` would resolve to the `Arc`'s own impl and hand back a
/// `&dyn Any` describing the `Arc` — downcasting which silently turns every read into a miss.
/// Deref first so `as_any` comes from the value inside.
fn downcast_cacheable<T>(val: Arc<dyn Cacheable>) -> Option<T>
where
    T: Clone + Cacheable,
{
    (*val).as_any().downcast_ref::<T>().cloned()
}

impl Cache {
    /// With given `time_to_live` and `time_to_idle` creates a moka cache.
    ///
    /// `name`        : Cache type name to be used as an attribute in metrics
    /// `time_to_live`: Time in seconds before an object is stored in a caching system before it’s deleted
    /// `time_to_idle`: Time in seconds before a `get` or `insert` operation an object is stored in a caching system before it's deleted
    /// `max_entries` : Maximum number of entries the cache can hold, `None` for unbounded
    pub fn new(
        name: &'static str,
        time_to_live: u64,
        time_to_idle: u64,
        max_entries: Option<u64>,
        populate_timeout: Duration,
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
            .time_to_live(Duration::from_secs(time_to_live))
            .time_to_idle(Duration::from_secs(time_to_idle))
            .eviction_listener(eviction_listener);

        // No weigher is configured, so moka counts entries — which is what this number has
        // always meant, and now says.
        if let Some(entries) = max_entries {
            cache_builder = cache_builder.max_capacity(entries);
        }

        Self {
            name,
            inner: cache_builder.build(),
            populate_timeout,
            loads: Mutex::new(HashMap::new()),
        }
    }

    fn record_populate_timeout(&self) {
        metrics::IN_MEMORY_CACHE_POPULATE_TIMEOUT
            .add(1, router_env::metric_attributes!(("cache_type", self.name)));
        logger::warn!(
            cache_type = self.name,
            timeout_secs = self.populate_timeout.as_secs(),
            "An in-memory cache populate attempt was cut off at the configured cap"
        );
    }

    fn record_population_avoided(&self) {
        metrics::IN_MEMORY_CACHE_POPULATION_AVOIDED
            .add(1, router_env::metric_attributes!(("cache_type", self.name)));
    }

    fn record_type_mismatch(&self) {
        metrics::IN_MEMORY_CACHE_TYPE_MISMATCH
            .add(1, router_env::metric_attributes!(("cache_type", self.name)));
        logger::error!(
            cache_type = self.name,
            "An in-memory cache entry could not be downcast to the expected type"
        );
    }

    /// Read a cached value or run `populate` on a miss.
    ///
    /// Loads for a key run one at a time. Each has its own error and timeout;
    /// time spent waiting for another load does not count toward the timeout.
    pub async fn get_or_populate_in_memory<T, Fut, E>(
        &self,
        key: CacheKey,
        populate: Fut,
    ) -> Result<T, E>
    where
        T: Cacheable + serde::Serialize + serde::de::DeserializeOwned + Clone,
        Fut: futures::Future<Output = Result<T, E>> + Send,
        E: From<StorageError> + Send + Sync + 'static,
    {
        self.get_or_populate_cacheable(key, async {
            populate.await.map(|value| CachePopulation {
                value,
                cacheable: true,
            })
        })
        .await
    }

    async fn get_or_populate_cacheable<T, Fut, E>(
        &self,
        key: CacheKey,
        populate: Fut,
    ) -> Result<T, E>
    where
        T: Cacheable + serde::Serialize + serde::de::DeserializeOwned + Clone,
        Fut: futures::Future<Output = Result<CachePopulation<T>, E>> + Send,
        E: From<StorageError> + Send + Sync + 'static,
    {
        // Cache hits skip load coordination.
        if let Some(val) = self.get_val::<T>(key.clone()).await {
            return Ok(val);
        }

        let moka_key = in_memory_cache_key(key.clone());
        let load = CacheLoad::new(self, &moka_key);
        let _serial = load.state.serial.lock().await;

        {
            let mut can_publish = load.state.can_publish.lock().await;
            // Another load or push may have filled the cache while we waited.
            if let Some(value) = self.inner.get(&moka_key).await {
                self.record_population_avoided();
                return downcast_cacheable::<T>(value).ok_or_else(|| {
                    self.record_type_mismatch();
                    StorageError::CacheTypeMismatch.into()
                });
            }
            *can_publish = true;
        }

        // Allow push/remove while the backend load is pending.
        let CachePopulation { value, cacheable } =
            match tokio::time::timeout(self.populate_timeout, populate).await {
                Ok(result) => result?,
                Err(_) => {
                    self.record_populate_timeout();
                    return Err(StorageError::CachePopulateTimedOut.into());
                }
            };

        if cacheable {
            let can_publish = load.state.can_publish.lock().await;
            if *can_publish {
                self.inner.insert(moka_key, Arc::new(value.clone())).await;
                drop(can_publish);
                self.record_populate_insert(key).await;
            }
        }
        Ok(value)
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
        let moka_key = in_memory_cache_key(key);
        let load = CacheLoad::new(self, &moka_key);
        let mut can_publish = load.state.can_publish.lock().await;
        *can_publish = false;
        self.inner.insert(moka_key, Arc::new(val)).await;
    }

    /// Record a cache fill as `in_memory_push` for Deja replay.
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
    async fn record_populate_insert(&self, key: CacheKey) {
        let _ = key;
    }

    // Deja: the L1 seam, instrumented on the method itself so no call path can
    // bypass it. A recorded `Some(v)` substitutes on replay; a recorded `None`
    // re-triggers the caller's fallback. The serde bound is deliberately
    // unconditional: a type that cannot be captured cannot be cached.
    //
    // `on_miss = None`: an unrecorded read reports "not in cache", which is true of
    // the cold replay cache, so the caller falls back instead of fail-stopping.
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
            on_miss = None,
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

        val.and_then(downcast_cacheable::<T>)
    }

    /// Check if a key exists in cache
    //
    // Deja: `on_miss = false` is the same "not in cache" answer as `get_val`.
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
            on_miss = false,
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
    /// Remove the entry and prevent its pending load from caching a stale value.
    pub async fn remove(&self, key: CacheKey) {
        let moka_key = in_memory_cache_key(key);
        let load = CacheLoad::new(self, &moka_key);
        let mut can_publish = load.state.can_publish.lock().await;
        *can_publish = false;
        self.inner.invalidate::<String>(&moka_key).await;
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

    /// The longest an entry can live here, when a time to live is configured.
    ///
    /// `time_to_live`, not `time_to_idle`: eviction takes whichever falls first, but the TTL is
    /// the bound a layer behind this one has to outlast.
    fn time_to_live(&self) -> Option<Duration> {
        self.inner.policy().time_to_live()
    }

    /// The redis lifetime that lets redis, rather than the database, absorb a miss from this
    /// cache.
    ///
    /// `None` when this cache has no TTL to scale or the product overflows, leaving the write on
    /// the connection's configured `default_ttl`.
    fn redis_ttl_for(&self, key: &str) -> Option<i64> {
        let ttl = self.time_to_live()?.as_secs();
        let ttl = i64::try_from(ttl.checked_mul(u64::from(REDIS_TTL_MULTIPLE))?).ok()?;

        Some(Self::jittered_ttl(key, ttl))
    }

    /// Extends `ttl` by up to [`REDIS_TTL_JITTER_MAX_PERCENT`], by an amount derived from `key`.
    ///
    /// Entries populated together — after a deploy, a flush, or a cold redis — otherwise share
    /// one expiry instant and fall through to the database as a herd.
    ///
    /// Keyed rather than random, so the TTL is identical across nodes and reproducible under
    /// deja replay, where `ttl_seconds` is a recorded boundary argument. Only ever added, so the
    /// entry outlives `ttl`; a `ttl` too small or too large to jitter is returned unchanged.
    fn jittered_ttl(key: &str, ttl: i64) -> i64 {
        let Some(scaled_by_percent) = ttl.checked_mul(i64::from(REDIS_TTL_JITTER_MAX_PERCENT))
        else {
            return ttl;
        };
        let Ok(spread) = u32::try_from(scaled_by_percent / 100) else {
            return ttl;
        };
        let Some(buckets) = spread.checked_add(1) else {
            return ttl;
        };

        ttl.saturating_add(i64::from(crc32fast::hash(key.as_bytes()) % buckets))
    }

    /// Records everything moka exposes about this cache's occupancy.
    ///
    /// `entry_count` and `weighted_size` are the only runtime figures it publishes — there are
    /// no built-in hit or miss statistics, which is why those are counted by hand in
    /// [`Self::get_val`]. The configured ceiling is recorded alongside so that utilisation is
    /// derivable from the metrics rather than from configuration.
    pub async fn record_size_metrics(&self) {
        self.run_pending_tasks().await;

        let attributes = router_env::metric_attributes!(("cache_type", self.name));

        metrics::IN_MEMORY_CACHE_ENTRY_COUNT.record(self.get_entry_count(), attributes);

        // `weighted_size` is deliberately not recorded: with no weigher configured it is the
        // entry count again, so it would be a second name for the gauge above.
        //
        // Absent for an unbounded cache, where there is no ceiling to be a fraction of.
        if let Some(max_capacity) = self.inner.policy().max_capacity() {
            metrics::IN_MEMORY_CACHE_MAX_CAPACITY.record(max_capacity, attributes);
        }
    }
}

#[instrument(skip_all)]
async fn get_or_populate_redis<T, Fut>(
    redis: &RedisConnectionWithContext,
    key: &str,
    ttl: Option<i64>,
    populate: Fut,
) -> CustomResult<CachePopulation<T>, StorageError>
where
    T: serde::Serialize + serde::de::DeserializeOwned + Debug,
    Fut: futures::Future<Output = CustomResult<T, StorageError>> + Send,
{
    let mut reservation = common_utils::generate_random_bytes(17);
    if let Some(prefix) = reservation.first_mut() {
        *prefix = 0;
    }
    let previous = match redis
        .get_key_or_reserve(&key.into(), &reservation, ttl)
        .await
    {
        Ok(previous) => previous,
        Err(error) => {
            // SET may fail at Redis's memory limit while GET still succeeds.
            if let Ok(bytes) = Box::pin(redis.get_key::<Vec<u8>>(&key.into())).await {
                if let Ok(value) = serde_json::from_slice::<T>(&bytes) {
                    return Ok(CachePopulation {
                        value,
                        cacheable: true,
                    });
                }
            }
            return Err(error.change_context(StorageError::KVError));
        }
    };
    let mut token = Cow::Borrowed(reservation.as_slice());
    if let Some(bytes) = previous {
        if bytes.len() == reservation.len() && bytes.first() == Some(&0) {
            token = Cow::Owned(bytes);
        } else if let Ok(value) = serde_json::from_slice::<T>(&bytes) {
            return Ok(CachePopulation {
                value,
                cacheable: true,
            });
        } else {
            // Box this uncommon path to keep the cache future small.
            let reserved = Box::pin(redis.reserve_cache_if_unchanged(
                &key.into(),
                &bytes,
                token.as_ref(),
                ttl,
            ))
            .await
            .change_context(StorageError::KVError)?;
            if !reserved {
                return Ok(CachePopulation {
                    value: populate.await?,
                    cacheable: false,
                });
            }
        }
    }
    let data = populate.await?;
    let value = serde_json::to_vec(&data)
        .change_context(RedisError::JsonSerializationFailed)
        .change_context(StorageError::KVError)?;
    let cacheable = redis
        .compare_and_set_cache(&key.into(), &token, &value)
        .await
        .change_context(StorageError::KVError)?;
    Ok(CachePopulation {
        value: data,
        cacheable,
    })
}

/// Recorded args for the [`Cache`] boundaries: cache name + un-namespaced
/// physical key, so the args a replay computes match the recording's.
#[cfg(feature = "deja")]
fn deja_in_memory_args(cache_name: &str, key: &CacheKey) -> serde_json::Value {
    serde_json::json!({ "cache": cache_name, "key": String::from(key.clone()) })
}

#[instrument(skip_all)]
pub async fn get_or_populate_in_memory_redis<T, Fut, S>(
    store: &S,
    key: &str,
    fun: Fut,
    cache: CacheId,
) -> CustomResult<T, StorageError>
where
    T: Cacheable + serde::Serialize + serde::de::DeserializeOwned + Debug + Clone,
    Fut: futures::Future<Output = CustomResult<T, StorageError>> + Send,
    S: RedisConnInterface + CacheInterface + Send + Sync + ?Sized,
{
    // Keep the fallback in this frame so nested cache futures only borrow it.
    let fun = std::pin::pin!(fun);
    let cache = store.caches().get(cache);
    let cache_key = CacheKey {
        key: key.to_string(),
        prefix: store.cache_key_prefix().to_string(),
    };
    // Redis is the layer behind this one, so it is held past this cache's own lifetime.
    let redis_ttl = cache.redis_ttl_for(key);

    // The redis connection is acquired only when this caller is the one populating, so an
    // in-memory hit answers during a redis outage rather than erroring, and concurrent
    // misses for one key share one Redis lookup and, if needed, one DB population.
    cache
        .get_or_populate_cacheable(cache_key, async {
            let redis = store
                .get_redis_conn()
                .change_context(StorageError::RedisError(
                    RedisError::RedisConnectionError.into(),
                ))
                .attach_printable("Failed to get redis connection")?;
            // Keep the larger Redis future off the in-memory hit path.
            Box::pin(get_or_populate_redis(&redis, key, redis_ttl, fun)).await
        })
        .await
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
    use std::{
        collections::HashSet,
        sync::atomic::{AtomicUsize, Ordering},
        time,
    };

    use super::*;

    /// Satisfies `get_or_populate_in_memory`'s `E: From<StorageError>` bound for these tests' `E = String`.
    impl From<StorageError> for String {
        fn from(err: StorageError) -> Self {
            err.to_string()
        }
    }

    /// Long enough that a correctly coalescing test never trips it, short enough that a test
    /// asserting the cap does not drag.
    const TEST_POPULATE_TIMEOUT: Duration = Duration::from_millis(500);

    fn test_key(key: &str) -> CacheKey {
        CacheKey {
            key: key.to_string(),
            prefix: "prefix".to_string(),
        }
    }

    /// A populate that counts its own invocations, so a test can assert how many callers
    /// actually reached the backend.
    fn counting_populate(
        calls: &Arc<AtomicUsize>,
        delay: Duration,
        value: impl Into<String>,
    ) -> futures::future::BoxFuture<'static, Result<String, String>> {
        let calls = Arc::clone(calls);
        let value = value.into();
        Box::pin(async move {
            calls.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(delay).await;
            Ok(value)
        })
    }

    /// A populate that always fails, counting its own invocations the same way.
    fn failing_populate(
        calls: &Arc<AtomicUsize>,
        delay: Duration,
        message: impl Into<String>,
    ) -> futures::future::BoxFuture<'static, Result<String, String>> {
        let calls = Arc::clone(calls);
        let message = message.into();
        Box::pin(async move {
            calls.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(delay).await;
            Err(message)
        })
    }

    #[tokio::test]
    async fn concurrent_misses_for_one_key_populate_once() {
        let cache = Arc::new(Cache::new("test", 1800, 1800, None, TEST_POPULATE_TIMEOUT));
        let calls = Arc::new(AtomicUsize::new(0));

        let readers = (0..20).map(|_| {
            let cache = Arc::clone(&cache);
            let populate = counting_populate(&calls, Duration::from_millis(50), "val");
            tokio::spawn(async move {
                cache
                    .get_or_populate_in_memory(test_key("key"), populate)
                    .await
            })
        });

        for result in futures::future::join_all(readers).await {
            assert_eq!(result.unwrap(), Ok("val".to_string()));
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn a_reader_arriving_during_a_populate_waits_and_hits() {
        let cache = Arc::new(Cache::new("test", 1800, 1800, None, TEST_POPULATE_TIMEOUT));
        let populating_calls = Arc::new(AtomicUsize::new(0));
        let reader_calls = Arc::new(AtomicUsize::new(0));

        let populating = {
            let cache = Arc::clone(&cache);
            let populate = counting_populate(&populating_calls, Duration::from_millis(100), "val");
            tokio::spawn(async move {
                cache
                    .get_or_populate_in_memory(test_key("key"), populate)
                    .await
            })
        };

        // Let the first caller take the write side before the second one arrives.
        tokio::time::sleep(Duration::from_millis(20)).await;

        let reader = {
            let cache = Arc::clone(&cache);
            let populate = counting_populate(&reader_calls, Duration::ZERO, "other");
            tokio::spawn(async move {
                cache
                    .get_or_populate_in_memory(test_key("key"), populate)
                    .await
            })
        };

        assert_eq!(populating.await.unwrap(), Ok("val".to_string()));
        assert_eq!(reader.await.unwrap(), Ok("val".to_string()));
        assert_eq!(populating_calls.load(Ordering::SeqCst), 1);
        // The reader never reached the backend — it read what the populate wrote.
        assert_eq!(reader_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn reads_of_a_populated_key_do_not_block_each_other() {
        let cache = Arc::new(Cache::new("test", 1800, 1800, None, TEST_POPULATE_TIMEOUT));
        cache.push(test_key("key"), "val".to_string()).await;
        let calls = Arc::new(AtomicUsize::new(0));

        // Far more concurrent readers than the runtime has worker threads: if the read side
        // serialized them, this could not finish well inside the wait budget.
        let started = time::Instant::now();
        let readers = (0..200).map(|_| {
            let cache = Arc::clone(&cache);
            let populate = counting_populate(&calls, Duration::ZERO, "other");
            tokio::spawn(async move {
                cache
                    .get_or_populate_in_memory(test_key("key"), populate)
                    .await
            })
        });

        for result in futures::future::join_all(readers).await {
            assert_eq!(result.unwrap(), Ok("val".to_string()));
        }
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(started.elapsed() < TEST_POPULATE_TIMEOUT);
    }

    #[tokio::test]
    async fn a_capped_populate_fails_that_caller_while_a_queued_caller_proceeds_independently() {
        // A cap far below the populate's delay, so the first caller is guaranteed to be capped.
        let cache = Arc::new(Cache::new(
            "test",
            1800,
            1800,
            None,
            Duration::from_millis(50),
        ));
        let slow_calls = Arc::new(AtomicUsize::new(0));
        let queued_calls = Arc::new(AtomicUsize::new(0));

        let slow = {
            let cache = Arc::clone(&cache);
            let populate = counting_populate(&slow_calls, Duration::from_millis(600), "slow");
            tokio::spawn(async move {
                cache
                    .get_or_populate_in_memory(test_key("key"), populate)
                    .await
            })
        };

        // Let the first caller start (and be capped) before the second one arrives.
        tokio::time::sleep(Duration::from_millis(20)).await;

        let queued = {
            let cache = Arc::clone(&cache);
            let populate = counting_populate(&queued_calls, Duration::ZERO, "own");
            tokio::spawn(async move {
                cache
                    .get_or_populate_in_memory(test_key("key"), populate)
                    .await
            })
        };

        // Cut off at the cap, no retry: invoked once despite never completing.
        assert_eq!(
            slow.await.unwrap(),
            Err(StorageError::CachePopulateTimedOut.to_string())
        );
        assert_eq!(slow_calls.load(Ordering::SeqCst), 1);

        // The failed turn still ended at the cap, so the queued caller gets its own turn
        // against an empty cache and populates for itself.
        assert_eq!(queued.await.unwrap(), Ok("own".to_string()));
        assert_eq!(queued_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn a_populate_failure_fails_only_that_caller_not_a_queued_one() {
        let cache = Arc::new(Cache::new("test", 1800, 1800, None, TEST_POPULATE_TIMEOUT));
        let failing_calls = Arc::new(AtomicUsize::new(0));
        let queued_calls = Arc::new(AtomicUsize::new(0));

        let failing = {
            let cache = Arc::clone(&cache);
            // Delayed, so the second caller below arrives while this is still the active
            // turn, not after it has already finished and freed the key.
            let populate = failing_populate(&failing_calls, Duration::from_millis(100), "boom");
            tokio::spawn(async move {
                cache
                    .get_or_populate_in_memory(test_key("key"), populate)
                    .await
            })
        };

        tokio::time::sleep(Duration::from_millis(20)).await;

        let queued_started = time::Instant::now();
        let queued = {
            let cache = Arc::clone(&cache);
            let populate = counting_populate(&queued_calls, Duration::ZERO, "own");
            tokio::spawn(async move {
                cache
                    .get_or_populate_in_memory(test_key("key"), populate)
                    .await
            })
        };

        assert_eq!(failing.await.unwrap(), Err("boom".to_string()));
        assert_eq!(failing_calls.load(Ordering::SeqCst), 1);

        // Proves real queuing rather than lucky scheduling: the second caller's own populate
        // has no delay, so an uncontended run would resolve in well under 50ms.
        assert_eq!(queued.await.unwrap(), Ok("own".to_string()));
        assert!(queued_started.elapsed() >= Duration::from_millis(50));
        assert_eq!(queued_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn distinct_keys_never_block_each_other() {
        let cache = Arc::new(Cache::new("test", 1800, 1800, None, TEST_POPULATE_TIMEOUT));
        let calls = Arc::new(AtomicUsize::new(0));

        let populates = (0..8).map(|index| {
            let cache = Arc::clone(&cache);
            let populate =
                counting_populate(&calls, Duration::from_millis(50), format!("val{index}"));
            tokio::spawn(async move {
                cache
                    .get_or_populate_in_memory(test_key(&format!("key{index}")), populate)
                    .await
            })
        });

        for (index, result) in futures::future::join_all(populates)
            .await
            .into_iter()
            .enumerate()
        {
            assert_eq!(result.unwrap(), Ok(format!("val{index}")));
        }
        // Every key is distinct, so every one of them populates.
        assert_eq!(calls.load(Ordering::SeqCst), 8);
    }

    /// The `[cache]` documentation in `config/*.toml` promises this syntax.
    #[test]
    fn the_documented_toml_syntax_deserializes() {
        let config: CacheConfig = config::Config::builder()
            .add_source(config::File::from_str(
                r#"
                invalidation_channel = "channel_from_toml"

                [accounts]
                ttl_in_secs = 120
                max_entries = 500000

                [config]
                max_entries = 0
                "#,
                config::FileFormat::Toml,
            ))
            .build()
            .expect("failed to build cache configuration")
            .try_deserialize()
            .expect("failed to deserialize cache configuration from TOML");

        assert_eq!(
            config.invalidation_channel.as_deref(),
            Some("channel_from_toml")
        );
        assert_eq!(config.accounts.time_to_live(), 120);
        assert_eq!(config.accounts.max_entries, Some(500_000));
        assert_eq!(config.config.max_entries, Some(0));
        // Untouched caches keep their own defaults.
        assert_eq!(config.routing.max_entries, None);
        assert_eq!(config.routing.time_to_live(), DEFAULT_CACHE_TTL);
    }

    /// Pinned against the same `Environment` source `Settings::with_config_path` builds.
    ///
    /// The registered list-parse keys are load-bearing: without any of them `list_separator`
    /// applies to every key and silently turns plain string values into single-element arrays,
    /// which no string field will accept.
    #[test]
    fn the_documented_env_vars_deserialize() {
        let source: HashMap<String, String> = [
            ("ROUTER__ACCOUNTS__MAX_ENTRIES", "500000"),
            ("ROUTER__CONFIG__MAX_ENTRIES", "0"),
            ("ROUTER__CGRAPH__TTL_IN_SECS", "120"),
            ("ROUTER__INVALIDATION_CHANNEL", "channel_from_env"),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();

        let config: CacheConfig = config::Config::builder()
            .add_source(
                config::Environment::with_prefix("ROUTER")
                    .try_parsing(true)
                    .separator("__")
                    .list_separator(",")
                    .with_list_parse_key("log.telemetry.route_to_trace")
                    .with_list_parse_key("redis.cluster_urls")
                    .source(Some(source)),
            )
            .build()
            .expect("failed to build cache configuration")
            .try_deserialize()
            .expect("failed to deserialize cache configuration from the environment");

        assert_eq!(config.accounts.max_entries, Some(500_000));
        assert_eq!(config.config.max_entries, Some(0));
        assert_eq!(config.cgraph.time_to_live(), 120);
        assert_eq!(
            config.invalidation_channel.as_deref(),
            Some("channel_from_env")
        );
    }

    #[test]
    fn redis_is_held_past_the_in_memory_lifetime() {
        let ttl_in_secs = 1800;
        let cache = Cache::new("test", ttl_in_secs, 600, None, Duration::from_secs(5));

        let redis_ttl = cache
            .redis_ttl_for("merchant_1")
            .expect("a cache built with a ttl reports one");

        // Doubled, then jittered upward: 3600s plus up to a tenth of it.
        assert!(
            (3600..=3960).contains(&redis_ttl),
            "redis ttl {redis_ttl} outside the jittered band"
        );

        // The property the multiple exists for, stated where retuning either number breaks it.
        let ttl_in_secs = i64::try_from(ttl_in_secs).expect("test ttl fits an i64");
        assert!(redis_ttl > ttl_in_secs);
    }

    #[test]
    fn jitter_stays_within_the_upper_decile() {
        let ttl = 300;
        for i in 0..1000 {
            let jittered = Cache::jittered_ttl(&format!("merchant_{i}"), ttl);
            assert!(
                (ttl..=ttl + ttl * i64::from(REDIS_TTL_JITTER_MAX_PERCENT) / 100)
                    .contains(&jittered),
                "ttl {jittered} out of range for merchant_{i}"
            );
        }
    }

    #[test]
    fn jitter_spreads_keys_and_repeats_for_one_key() {
        let ttl = 300;
        let jittered = (0..1000)
            .map(|i| Cache::jittered_ttl(&format!("merchant_{i}"), ttl))
            .collect::<HashSet<_>>();

        // A 300s ttl offers 31 distinct expiries; 1000 keys should reach most of them.
        assert!(jittered.len() > 25, "keys bunched onto {jittered:?}");

        // Same key, same ttl — this is what keeps a deja replay byte-identical.
        assert_eq!(
            Cache::jittered_ttl("merchant_1", ttl),
            Cache::jittered_ttl("merchant_1", ttl)
        );
    }

    #[test]
    fn ttl_too_small_or_negative_is_left_alone() {
        for ttl in [-1, 0, 1, 9] {
            assert_eq!(Cache::jittered_ttl("key", ttl), ttl);
        }
    }

    #[tokio::test]
    async fn construct_and_get_cache() {
        let cache = Cache::new("test", 1800, 1800, None, TEST_POPULATE_TIMEOUT);
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
        let cache = Cache::new("test", 2, 2, Some(0), TEST_POPULATE_TIMEOUT);
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
        let cache = Cache::new("test", 1800, 1800, None, TEST_POPULATE_TIMEOUT);
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
        let cache = Cache::new("test", 2, 2, None, TEST_POPULATE_TIMEOUT);
        cache
            .push(
                CacheKey {
                    key: "key".to_string(),
                    prefix: "prefix".to_string(),
                },
                "val".to_string(),
            )
            .await;
        tokio::time::sleep(Duration::from_secs(3)).await;
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
    async fn invalidation_during_load() {
        for (invalidated_key, expected) in [("a", None), ("b", Some(1))] {
            let cache = Cache::new("test", 60, 60, None, TEST_POPULATE_TIMEOUT);
            let (started_tx, started_rx) = tokio::sync::oneshot::channel();
            let (resume_tx, resume_rx) = tokio::sync::oneshot::channel();
            let mut reader = Box::pin(cache.get_or_populate_in_memory(test_key("a"), async {
                started_tx.send(()).unwrap();
                resume_rx.await.unwrap();
                Ok::<_, StorageError>(1_usize)
            }));
            tokio::select! {
                result = &mut reader => panic!("load finished before release: {result:?}"),
                result = started_rx => result.unwrap(),
            }
            tokio::time::timeout(
                Duration::from_millis(100),
                cache.remove(test_key(invalidated_key)),
            )
            .await
            .expect("invalidation must not wait for the paused loader");
            resume_tx.send(()).unwrap();
            assert_eq!(reader.await.unwrap(), 1);
            assert_eq!(cache.get_val::<usize>(test_key("a")).await, expected);
        }
    }

    async fn wait_for_registered_loaders(cache: &Cache, callers: usize) {
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if cache
                    .loads
                    .lock()
                    .unwrap()
                    .values()
                    .any(|state| state.users == callers)
                {
                    return;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("callers registered before cancellation/invalidation");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn cancelled_loads_release_state() {
        for queued_count in [0, 32] {
            let cache = Arc::new(Cache::new("test", 60, 60, None, Duration::from_secs(10)));
            let (started_tx, started_rx) = tokio::sync::oneshot::channel();
            let mut active = Box::pin(cache.get_or_populate_in_memory(
                test_key("lifecycle"),
                async {
                    started_tx.send(()).unwrap();
                    futures::future::pending::<Result<usize, StorageError>>().await
                },
            ));
            tokio::select! {
                result = &mut active => panic!("pending load completed: {result:?}"),
                result = started_rx => result.unwrap(),
            }
            let fresh_loads = Arc::new(AtomicUsize::new(0));
            let mut queued = (0..queued_count)
                .map(|_| {
                    let cache = cache.clone();
                    let fresh_loads = fresh_loads.clone();
                    tokio::spawn(async move {
                        cache
                            .get_or_populate_in_memory(test_key("lifecycle"), async {
                                fresh_loads.fetch_add(1, Ordering::SeqCst);
                                Ok::<_, StorageError>(2_usize)
                            })
                            .await
                    })
                })
                .collect::<Vec<_>>();
            wait_for_registered_loaders(&cache, queued_count + 1).await;
            for task in queued.drain(..queued_count / 2) {
                task.abort();
                assert!(task.await.unwrap_err().is_cancelled());
            }
            wait_for_registered_loaders(&cache, queued_count / 2 + 1).await;
            drop(active);
            for task in queued {
                assert_eq!(task.await.unwrap().unwrap(), 2);
            }
            assert!(cache.loads.lock().unwrap().is_empty());
            assert_eq!(
                cache
                    .get_or_populate_in_memory(test_key("lifecycle"), async {
                        fresh_loads.fetch_add(1, Ordering::SeqCst);
                        Ok::<_, StorageError>(2_usize)
                    })
                    .await
                    .unwrap(),
                2
            );
            assert_eq!(fresh_loads.load(Ordering::SeqCst), 1);
            assert_eq!(cache.get_val::<usize>(test_key("lifecycle")).await, Some(2));
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn invalidation_allows_queued_load() {
        let cache = Arc::new(Cache::new("test", 60, 60, None, Duration::from_secs(10)));
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel();
        let mut active = Box::pin(cache.get_or_populate_in_memory(test_key("key"), async {
            started_tx.send(()).unwrap();
            release_rx.await.unwrap();
            Ok::<_, StorageError>(1_usize)
        }));
        tokio::select! {
            result = &mut active => panic!("load finished before release: {result:?}"),
            result = started_rx => result.unwrap(),
        }

        let queued_cache = cache.clone();
        let queued = tokio::spawn(async move {
            queued_cache
                .get_or_populate_in_memory(test_key("key"), async {
                    Ok::<_, StorageError>(2_usize)
                })
                .await
        });
        wait_for_registered_loaders(&cache, 2).await;
        tokio::time::timeout(Duration::from_secs(1), cache.remove(test_key("key")))
            .await
            .expect("invalidation must not wait for the paused loader");

        release_tx.send(()).unwrap();
        assert_eq!(active.await.unwrap(), 1);
        assert_eq!(queued.await.unwrap().unwrap(), 2);
        assert_eq!(cache.get_val::<usize>(test_key("key")).await, Some(2));
        assert!(cache.loads.lock().unwrap().is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn push_overrides_pending_load() {
        let cache = Arc::new(Cache::new("test", 60, 60, None, Duration::from_secs(10)));
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel();
        let mut active = Box::pin(cache.get_or_populate_in_memory(test_key("key"), async {
            started_tx.send(()).unwrap();
            release_rx.await.unwrap();
            Ok::<_, StorageError>(1_usize)
        }));
        tokio::select! {
            result = &mut active => panic!("load finished before release: {result:?}"),
            result = started_rx => result.unwrap(),
        }

        let queued_cache = cache.clone();
        let queued = tokio::spawn(async move {
            queued_cache
                .get_or_populate_in_memory::<usize, _, StorageError>(test_key("key"), async {
                    panic!("queued reader should use the pushed value")
                })
                .await
        });
        wait_for_registered_loaders(&cache, 2).await;
        tokio::time::timeout(Duration::from_secs(1), cache.push(test_key("key"), 2_usize))
            .await
            .expect("push must not wait for the paused loader");

        release_tx.send(()).unwrap();
        assert_eq!(active.await.unwrap(), 1);
        assert_eq!(queued.await.unwrap().unwrap(), 2);
        assert_eq!(cache.get_val::<usize>(test_key("key")).await, Some(2));
        assert!(cache.loads.lock().unwrap().is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[ignore = "requires Redis; set HYPERSWITCH_REDIS_REPRO_PORT"]
    async fn redis_recovery_respects_updates() {
        type DecodeGate = (
            tokio::sync::oneshot::Sender<()>,
            std::sync::mpsc::Receiver<()>,
        );
        static DECODE_GATE: Mutex<Option<DecodeGate>> = Mutex::new(None);

        #[derive(Debug, serde::Serialize)]
        #[serde(transparent)]
        struct PausedValue(String);

        impl<'de> serde::Deserialize<'de> for PausedValue {
            // Broken synchronization must fail this test, not resemble malformed data.
            #[allow(clippy::unwrap_in_result)]
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let text = <String as serde::Deserialize>::deserialize(deserializer)?;
                if text == "bad-cache-entry" {
                    let (started, release) = DECODE_GATE.lock().unwrap().take().unwrap();
                    started.send(()).unwrap();
                    release.recv_timeout(Duration::from_secs(10)).unwrap();
                    Err(serde::de::Error::custom(
                        "intentionally malformed cache value",
                    ))
                } else {
                    Ok(Self(text))
                }
            }
        }

        let redis = test_redis_connection().await;
        for replacement in [
            None,
            Some(b"\"new-value\"".as_slice()),
            Some(b"\0bbbbbbbbbbbbbbbb".as_slice()),
        ] {
            let key = format!("malformed-recovery:{}", common_utils::generate_uuid_v4());
            redis
                .set_key_with_expiry(&key.as_str().into(), b"\"bad-cache-entry\"".as_slice(), 60)
                .await
                .unwrap();
            let (started_tx, started_rx) = tokio::sync::oneshot::channel();
            let (release_tx, release_rx) = std::sync::mpsc::channel();
            *DECODE_GATE.lock().unwrap() = Some((started_tx, release_rx));
            let load_redis = redis.clone();
            let load_key = key.clone();
            let load = tokio::spawn(async move {
                get_or_populate_redis(&load_redis, &load_key, Some(120), async {
                    Ok(PausedValue("old-value".to_owned()))
                })
                .await
                .unwrap()
            });
            tokio::time::timeout(Duration::from_secs(10), started_rx)
                .await
                .unwrap()
                .unwrap();
            redis.delete_key(&key.as_str().into()).await.unwrap();
            if let Some(value) = replacement {
                redis
                    .set_key_with_expiry(&key.as_str().into(), value, 60)
                    .await
                    .unwrap();
            }
            release_tx.send(()).unwrap();
            let result = load.await.unwrap();
            let stored = redis
                .get_key::<Vec<u8>>(&key.as_str().into())
                .await
                .unwrap();
            redis.delete_key(&key.as_str().into()).await.unwrap();
            assert_eq!(result.value.0, "old-value");
            assert_eq!(
                stored.as_slice(),
                replacement.unwrap_or_default(),
                "recovery overwrote invalidation/replacement"
            );
            assert!(
                !result.cacheable,
                "revoked malformed reader must not enter Moka"
            );
        }
    }

    async fn test_redis_connection() -> RedisConnectionWithContext {
        let settings = redis_interface::RedisSettings {
            port: std::env::var("HYPERSWITCH_REDIS_REPRO_PORT")
                .expect("set HYPERSWITCH_REDIS_REPRO_PORT to a disposable Redis port")
                .parse()
                .unwrap(),
            pool_size: 1,
            ..redis_interface::RedisSettings::default()
        };
        RedisConnectionWithContext::new_without_context(Arc::new(
            redis_interface::RedisConnectionPool::new_without_event_emitter(&settings)
                .await
                .unwrap(),
        ))
    }

    #[tokio::test]
    #[ignore = "requires Redis; set HYPERSWITCH_REDIS_REPRO_PORT"]
    async fn redis_invalidation_blocks_stale_fill() {
        tokio::time::timeout(Duration::from_secs(10), async {
            let redis = test_redis_connection().await;
            for ttl in [None, Some(60)] {
                let key = format!("cache-test:{}", common_utils::generate_uuid_v4());
                let cache_key = CacheKey {
                    key: key.clone(),
                    prefix: String::new(),
                };
                let cache = Cache::new("test", 60, 60, None, TEST_POPULATE_TIMEOUT);
                let (started_tx, started_rx) = tokio::sync::oneshot::channel();
                let (resume_tx, resume_rx) = tokio::sync::oneshot::channel();
                let mut reader =
                    Box::pin(cache.get_or_populate_cacheable(cache_key.clone(), async {
                        Box::pin(get_or_populate_redis(&redis, &key, ttl, async {
                            started_tx.send(()).unwrap();
                            resume_rx.await.unwrap();
                            Ok(1_usize)
                        }))
                        .await
                    }));
                tokio::select! {
                    result = &mut reader => panic!("load finished before release: {result:?}"),
                    result = started_rx => result.unwrap(),
                }
                // Revoke the Redis reservation while the old DB snapshot is in flight.
                // This must protect Moka even before a pub/sub handler runs.
                redis.delete_key(&key.as_str().into()).await.unwrap();
                resume_tx.send(()).unwrap();
                assert_eq!(reader.await.unwrap(), 1);
                assert!(cache.get_val::<usize>(cache_key.clone()).await.is_none());
                assert!(matches!(
                    redis
                        .get_and_deserialize_key::<usize>(&key.as_str().into(), "usize")
                        .await
                        .unwrap_err()
                        .current_context(),
                    RedisError::NotFound
                ));
                assert_eq!(
                    cache
                        .get_or_populate_cacheable(cache_key, async {
                            Box::pin(get_or_populate_redis(&redis, &key, ttl, async {
                                Ok(2_usize)
                            }))
                            .await
                        })
                        .await
                        .unwrap(),
                    2
                );
                assert!(redis.get_ttl(&key.as_str().into()).await.unwrap() > 0);
                redis.delete_key(&key.as_str().into()).await.unwrap();
            }
        })
        .await
        .expect("Redis invalidation test timed out");
    }

    #[tokio::test]
    #[ignore = "requires Redis; set HYPERSWITCH_REDIS_REPRO_PORT"]
    async fn redis_competing_fills() {
        tokio::time::timeout(Duration::from_secs(10), async {
            let redis = test_redis_connection().await;
            // Identical fills may share the result; different values must not
            // replace it, even when the original reservation has expired.
            for (expire, new_value) in [(false, 1_usize), (false, 2), (true, 2)] {
                let key = format!("cache-test:{}", common_utils::generate_uuid_v4());
                let (started_tx, started_rx) = tokio::sync::oneshot::channel();
                let (resume_tx, resume_rx) = tokio::sync::oneshot::channel();
                let mut reader = Box::pin(get_or_populate_redis(&redis, &key, Some(60), async {
                    started_tx.send(()).unwrap();
                    resume_rx.await.unwrap();
                    Ok(1_usize)
                }));
                tokio::select! {
                    _ = &mut reader => panic!("load finished before release"),
                    result = started_rx => result.unwrap(),
                }
                if expire {
                    redis.set_expiry(&key.as_str().into(), 0).await.unwrap();
                }
                let newer = get_or_populate_redis(&redis, &key, Some(60), async { Ok(new_value) })
                    .await
                    .unwrap();
                assert_eq!(newer.value, new_value);
                assert!(newer.cacheable);
                resume_tx.send(()).unwrap();
                let older = reader.await.unwrap();
                assert_eq!(older.value, 1);
                assert_eq!(older.cacheable, new_value == 1);
                assert_eq!(
                    redis
                        .get_and_deserialize_key::<usize>(&key.as_str().into(), "usize")
                        .await
                        .unwrap(),
                    new_value
                );
                redis.delete_key(&key.as_str().into()).await.unwrap();
            }
        })
        .await
        .expect("Redis replacement test timed out");
    }

    #[tokio::test]
    #[ignore = "requires disposable Redis"]
    async fn redis_subscriber_handles_pending_load() {
        let capacity = redis_interface::RedisSettings::default().broadcast_channel_capacity;
        let pool = test_redis_connection().await.redis_conn;
        let channel = format!("review-invalidation-{}", common_utils::generate_uuid_v4());
        let caches = Arc::new(Caches::new(&CacheConfig {
            invalidation_channel: Some(channel.clone()),
            ..CacheConfig::default()
        }));
        let key = |s: &str| CacheKey {
            key: s.to_owned(),
            prefix: String::new(),
        };
        caches.config.push(key("b"), 2_usize).await;
        pool.subscriber.subscribe(&channel).await.unwrap();
        let mut observer = pool.subscriber.message_rx();
        let mut handler = Box::pin(pool.on_message(Arc::clone(&caches)));
        assert!(futures::poll!(handler.as_mut()).is_pending());

        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (resume_tx, resume_rx) = tokio::sync::oneshot::channel();
        let mut reader = Box::pin(caches.config.get_or_populate_in_memory(key("a"), async {
            started_tx.send(()).unwrap();
            resume_rx.await.unwrap();
            Ok::<usize, StorageError>(1)
        }));
        tokio::select! {
            result = &mut reader => panic!("load finished before release: {result:?}"),
            result = started_rx => result.unwrap(),
        }
        pool.publish(&channel, CacheKind::Config("a".into()))
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(1), observer.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(futures::poll!(handler.as_mut()).is_pending());

        // Process more messages than the buffer holds while a's load is paused.
        for _ in 0..capacity + 8 {
            pool.publish(&channel, CacheKind::Config("b".into()))
                .await
                .unwrap();
            tokio::time::timeout(Duration::from_secs(1), observer.recv())
                .await
                .unwrap()
                .unwrap();
            assert!(futures::poll!(handler.as_mut()).is_pending());
        }
        assert_eq!(caches.config.get_val::<usize>(key("b")).await, None);
        resume_tx.send(()).unwrap();
        assert_eq!(reader.await.unwrap(), 1);
        let exited = tokio::time::timeout(Duration::from_millis(100), handler.as_mut())
            .await
            .is_ok();
        assert!(!exited, "subscriber must keep receiving after invalidation");
        assert_eq!(caches.config.get_val::<usize>(key("a")).await, None);
    }
}
