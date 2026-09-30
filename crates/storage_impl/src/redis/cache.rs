use std::{any::Any, borrow::Cow, fmt::Debug, sync::Arc, time::Duration};

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
use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};

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

/// Default budget a caller waits on another caller's population of the same key, 5 seconds.
///
/// Roughly two orders of magnitude above a healthy redis `GET` plus database `SELECT`, so
/// legitimately slow populations still coalesce, while a stuck backend is bounded well under
/// redis's own command timeout.
pub const DEFAULT_POPULATE_WAIT_TIMEOUT_IN_SECS: u64 = 5;

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

/// How long an unused per-key population lock is kept around.
///
/// Only has to outlive a population, which is bounded by the wait budget; the lower bound
/// keeps short budgets from evicting locks that are still in use.
fn populate_lock_time_to_idle(populate_wait: Duration) -> Duration {
    Duration::from_secs(300).max(populate_wait * 2)
}

/// The read side of a key's population lock, plus whether taking it meant waiting on a
/// population that was already in flight.
struct ReadPermit {
    _guard: Option<OwnedRwLockReadGuard<()>>,
    waited: bool,
}

impl ReadPermit {
    fn unheld(waited: bool) -> Self {
        Self {
            _guard: None,
            waited,
        }
    }
}

/// Upper bound on distinct keys holding a population lock at once. moka counts entries here,
/// and an `Arc<RwLock<()>>` is tiny, so this is generous by design.
const POPULATE_LOCK_MAX_ENTRIES: u64 = 10_000;

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
    /// Seconds a caller waits for another caller's population of the same key before giving
    /// up and populating the key itself. `0` disables the wait entirely.
    pub populate_wait_timeout_in_secs: Option<u64>,
}

impl CacheSettings {
    fn time_to_live(&self) -> u64 {
        self.ttl_in_secs.unwrap_or(DEFAULT_CACHE_TTL)
    }

    fn time_to_idle(&self) -> u64 {
        self.tti_in_secs.unwrap_or(DEFAULT_CACHE_TTI)
    }

    fn populate_wait(&self) -> Duration {
        Duration::from_secs(
            self.populate_wait_timeout_in_secs
                .unwrap_or(DEFAULT_POPULATE_WAIT_TIMEOUT_IN_SECS),
        )
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
    /// `default` is the cache's own ceiling when the configuration names none, `None` meaning
    /// unbounded.
    fn build(&self, name: &'static str, default: Option<u64>) -> Cache {
        Cache::new(
            name,
            self.time_to_live(),
            self.time_to_idle(),
            self.max_entries(default),
            self.populate_wait(),
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
            config: config.config.build("CONFIG_CACHE", UNBOUNDED),
            accounts: config.accounts.build("ACCOUNTS_CACHE", DEFAULT),
            mca_list: config.mca_list.build("MCA_LIST_CACHE", DEFAULT),
            routing: config.routing.build("ROUTING_CACHE", DEFAULT),
            decision_manager: config
                .decision_manager
                .build("DECISION_MANAGER_CACHE", DEFAULT),
            surcharge: config.surcharge.build("SURCHARGE_CACHE", DEFAULT),
            cgraph: config.cgraph.build("CGRAPH_CACHE", DEFAULT),
            pm_filters_cgraph: config
                .pm_filters_cgraph
                .build("PM_FILTERS_CGRAPH_CACHE", DEFAULT),
            success_based_dynamic_algorithm: config
                .success_based_dynamic_algorithm
                .build("SUCCESS_BASED_DYNAMIC_ALGORITHM_CACHE", DEFAULT),
            elimination_based_dynamic_algorithm: config
                .elimination_based_dynamic_algorithm
                .build("ELIMINATION_BASED_DYNAMIC_ALGORITHM_CACHE", DEFAULT),
            contract_based_dynamic_algorithm: config
                .contract_based_dynamic_algorithm
                .build("CONTRACT_BASED_DYNAMIC_ALGORITHM_CACHE", DEFAULT),
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
    /// Per-key population locks, keyed exactly as [`Self::inner`] is.
    ///
    /// Readers hold the read side and so run concurrently with each other; a population
    /// holds the write side and excludes them until the value is in place.
    ///
    /// Best effort: should an entry be evicted while its lock is held, a later arrival
    /// builds a fresh lock and populates concurrently. That costs a coalescing, never
    /// correctness — both populations write the same value.
    populate_locks: MokaCache<String, Arc<RwLock<()>>>,
    populate_wait: Duration,
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
    /// `max_entries` : Maximum number of entries the cache can hold, `None` for unbounded
    pub fn new(
        name: &'static str,
        time_to_live: u64,
        time_to_idle: u64,
        max_entries: Option<u64>,
        populate_wait: Duration,
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
            populate_locks: MokaCache::builder()
                .time_to_idle(populate_lock_time_to_idle(populate_wait))
                .max_capacity(POPULATE_LOCK_MAX_ENTRIES)
                .build(),
            populate_wait,
        }
    }

    /// The lock coordinating population of `key`, created on first use.
    async fn get_lock_for_cache_key(&self, key: &CacheKey) -> Arc<RwLock<()>> {
        self.populate_locks
            .get_with(in_memory_cache_key(key.clone()), async {
                Arc::new(RwLock::new(()))
            })
            .await
    }

    /// Takes the read side for `key`: concurrent with every other reader of it, held off
    /// only while a population owns the write side.
    ///
    /// Giving up is always safe. The lock coordinates callers, it does not protect the cache
    /// — moka is already thread-safe — so a caller that proceeds without it costs a
    /// coalescing and nothing else.
    async fn read_permit(&self, key: &CacheKey) -> ReadPermit {
        if self.populate_wait.is_zero() {
            return ReadPermit::unheld(false);
        }

        let lock = self.get_lock_for_cache_key(key).await;

        // The uncontended case settles here, without suspending: the read side is only ever
        // unavailable while a population holds the write side.
        if let Ok(guard) = Arc::clone(&lock).try_read_owned() {
            return ReadPermit {
                _guard: Some(guard),
                waited: false,
            };
        }

        match tokio::time::timeout(self.populate_wait, lock.read_owned()).await {
            Ok(guard) => ReadPermit {
                _guard: Some(guard),
                waited: true,
            },
            Err(_elapsed) => {
                self.record_population_wait_timeout();
                ReadPermit::unheld(true)
            }
        }
    }

    /// Takes the write side for `key`, excluding every reader of it until dropped.
    async fn populate_permit(&self, key: &CacheKey) -> Option<OwnedRwLockWriteGuard<()>> {
        if self.populate_wait.is_zero() {
            return None;
        }

        let lock = self.get_lock_for_cache_key(key).await;
        match tokio::time::timeout(self.populate_wait, lock.write_owned()).await {
            Ok(guard) => Some(guard),
            Err(_elapsed) => {
                self.record_population_wait_timeout();
                None
            }
        }
    }

    fn record_population_wait_timeout(&self) {
        metrics::IN_MEMORY_CACHE_POPULATION_WAIT_TIMEOUT
            .add(1, router_env::metric_attributes!(("cache_type", self.name)));
        logger::warn!(
            cache_type = self.name,
            wait_secs = self.populate_wait.as_secs(),
            "Timed out waiting on an in-memory cache population; proceeding independently"
        );
    }

    fn record_population_avoided(&self) {
        metrics::IN_MEMORY_CACHE_POPULATION_AVOIDED
            .add(1, router_env::metric_attributes!(("cache_type", self.name)));
    }

    /// Reads `key`, and on a miss populates it by running `populate`.
    ///
    /// Concurrent callers for one key are coordinated rather than serialized: readers run
    /// together, and while one of them is populating, the rest wait on the write side and
    /// then read the value it wrote instead of each running `populate` themselves. When the
    /// population finishes they are all released at once.
    ///
    /// Waits are bounded. A caller whose budget expires populates independently — which is
    /// exactly what it would have done without any of this — so a stuck population slows
    /// callers down but never strands them.
    pub async fn get_or_populate<T, F, Fut, E>(&self, key: CacheKey, populate: F) -> Result<T, E>
    where
        T: Cacheable + serde::Serialize + serde::de::DeserializeOwned + Clone,
        F: FnOnce() -> Fut + Send,
        Fut: futures::Future<Output = Result<T, E>> + Send,
    {
        {
            let permit = self.read_permit(&key).await;
            if let Some(val) = self.get_val::<T>(key.clone()).await {
                if permit.waited {
                    self.record_population_avoided();
                }
                return Ok(val);
            }
        }

        // The read guard has to be dropped before asking for the write side — tokio's
        // `RwLock` has no upgrade, and holding both would deadlock against ourselves. That
        // leaves a gap in which another caller may have populated the key, so re-check.
        let _populating = self.populate_permit(&key).await;
        if let Some(val) = self.get_val::<T>(key.clone()).await {
            return Ok(val);
        }

        let val = populate().await?;
        self.push(key, val.clone()).await;

        Ok(val)
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

        // The explicit deref is load-bearing. `Arc<dyn Cacheable>` satisfies the blanket
        // `Cacheable` impl itself, so `val?.as_any()` would resolve to the `Arc`'s own impl and
        // hand back a `&dyn Any` describing the `Arc` — downcasting which silently turns every
        // read into a miss. Deref first so `as_any` comes from the value inside.
        (*val?).as_any().downcast_ref::<T>().cloned()
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

    // The redis connection is acquired only when this caller is the one populating, so an
    // in-memory hit answers during a redis outage rather than erroring, and concurrent
    // misses for one key cost a single redis round trip between them. The round trip also
    // reports the payload size, so weighing the entry costs nothing extra.
    cache
        .get_or_populate(cache_key, || async {
            let redis = store
                .get_redis_conn()
                .change_context(StorageError::RedisError(
                    RedisError::RedisConnectionError.into(),
                ))
                .attach_printable("Failed to get redis connection")?;
            get_or_populate_redis(&redis, key, None, fun).await
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
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    /// Long enough that a correctly coalescing test never trips it, short enough that a test
    /// asserting the timeout does not drag.
    const TEST_POPULATE_WAIT: Duration = Duration::from_millis(500);

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
    ) -> impl FnOnce() -> futures::future::BoxFuture<'static, Result<String, String>> {
        let calls = Arc::clone(calls);
        let value = value.into();
        move || {
            Box::pin(async move {
                calls.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(delay).await;
                Ok(value)
            })
        }
    }

    #[tokio::test]
    async fn concurrent_misses_for_one_key_populate_once() {
        let cache = Arc::new(Cache::new("test", 1800, 1800, None, TEST_POPULATE_WAIT));
        let calls = Arc::new(AtomicUsize::new(0));

        let readers = (0..20).map(|_| {
            let cache = Arc::clone(&cache);
            let populate = counting_populate(&calls, Duration::from_millis(50), "val");
            tokio::spawn(async move { cache.get_or_populate(test_key("key"), populate).await })
        });

        for result in futures::future::join_all(readers).await {
            assert_eq!(result.unwrap(), Ok("val".to_string()));
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn a_reader_arriving_during_a_populate_waits_and_hits() {
        let cache = Arc::new(Cache::new("test", 1800, 1800, None, TEST_POPULATE_WAIT));
        let populating_calls = Arc::new(AtomicUsize::new(0));
        let reader_calls = Arc::new(AtomicUsize::new(0));

        let populating = {
            let cache = Arc::clone(&cache);
            let populate = counting_populate(&populating_calls, Duration::from_millis(100), "val");
            tokio::spawn(async move { cache.get_or_populate(test_key("key"), populate).await })
        };

        // Let the first caller take the write side before the second one arrives.
        tokio::time::sleep(Duration::from_millis(20)).await;

        let reader = {
            let cache = Arc::clone(&cache);
            let populate = counting_populate(&reader_calls, Duration::ZERO, "other");
            tokio::spawn(async move { cache.get_or_populate(test_key("key"), populate).await })
        };

        assert_eq!(populating.await.unwrap(), Ok("val".to_string()));
        assert_eq!(reader.await.unwrap(), Ok("val".to_string()));
        assert_eq!(populating_calls.load(Ordering::SeqCst), 1);
        // The reader never reached the backend — it read what the populate wrote.
        assert_eq!(reader_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn reads_of_a_populated_key_do_not_block_each_other() {
        let cache = Arc::new(Cache::new("test", 1800, 1800, None, TEST_POPULATE_WAIT));
        cache.push(test_key("key"), "val".to_string()).await;
        let calls = Arc::new(AtomicUsize::new(0));

        // Far more concurrent readers than the runtime has worker threads: if the read side
        // serialized them, this could not finish well inside the wait budget.
        let started = std::time::Instant::now();
        let readers = (0..200).map(|_| {
            let cache = Arc::clone(&cache);
            let populate = counting_populate(&calls, Duration::ZERO, "other");
            tokio::spawn(async move { cache.get_or_populate(test_key("key"), populate).await })
        });

        for result in futures::future::join_all(readers).await {
            assert_eq!(result.unwrap(), Ok("val".to_string()));
        }
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(started.elapsed() < TEST_POPULATE_WAIT);
    }

    #[tokio::test]
    async fn a_populate_that_overruns_the_budget_releases_waiting_readers() {
        // A budget far below how long the populate takes, so the waiter is guaranteed to give
        // up rather than racing the sleep.
        let cache = Arc::new(Cache::new(
            "test",
            1800,
            1800,
            None,
            Duration::from_millis(50),
        ));
        let slow_calls = Arc::new(AtomicUsize::new(0));
        let waiter_calls = Arc::new(AtomicUsize::new(0));

        let slow = {
            let cache = Arc::clone(&cache);
            let populate = counting_populate(&slow_calls, Duration::from_millis(600), "slow");
            tokio::spawn(async move { cache.get_or_populate(test_key("key"), populate).await })
        };

        tokio::time::sleep(Duration::from_millis(20)).await;

        let waiter = {
            let cache = Arc::clone(&cache);
            let populate = counting_populate(&waiter_calls, Duration::ZERO, "own");
            tokio::spawn(async move { cache.get_or_populate(test_key("key"), populate).await })
        };

        // The waiter gave up on the stuck population and fetched for itself rather than
        // blocking until the slow caller finished.
        assert_eq!(waiter.await.unwrap(), Ok("own".to_string()));
        assert_eq!(waiter_calls.load(Ordering::SeqCst), 1);

        assert_eq!(slow.await.unwrap(), Ok("slow".to_string()));
        assert_eq!(slow_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn distinct_keys_never_block_each_other() {
        let cache = Arc::new(Cache::new("test", 1800, 1800, None, TEST_POPULATE_WAIT));
        let calls = Arc::new(AtomicUsize::new(0));

        let populates = (0..8).map(|index| {
            let cache = Arc::clone(&cache);
            let populate =
                counting_populate(&calls, Duration::from_millis(50), format!("val{index}"));
            tokio::spawn(async move {
                cache
                    .get_or_populate(test_key(&format!("key{index}")), populate)
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

    /// The negative control for `concurrent_misses_for_one_key_populate_once`.
    ///
    /// Without it that test could pass for the wrong reason — tasks simply not overlapping.
    /// Here the coordination is switched off and the same shape produces one populate per
    /// caller, so the two together show the coalescing is what causes the difference.
    #[tokio::test]
    async fn a_zero_wait_budget_disables_coordination() {
        let cache = Arc::new(Cache::new("test", 1800, 1800, None, Duration::ZERO));
        let calls = Arc::new(AtomicUsize::new(0));

        let readers = (0..4).map(|_| {
            let cache = Arc::clone(&cache);
            let populate = counting_populate(&calls, Duration::from_millis(50), "val");
            tokio::spawn(async move { cache.get_or_populate(test_key("key"), populate).await })
        });

        for result in futures::future::join_all(readers).await {
            assert_eq!(result.unwrap(), Ok("val".to_string()));
        }
        assert_eq!(calls.load(Ordering::SeqCst), 4);
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
        let source: std::collections::HashMap<String, String> = [
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

    #[tokio::test]
    async fn construct_and_get_cache() {
        let cache = Cache::new("test", 1800, 1800, None, TEST_POPULATE_WAIT);
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
        let cache = Cache::new("test", 2, 2, Some(0), TEST_POPULATE_WAIT);
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
        let cache = Cache::new("test", 1800, 1800, None, TEST_POPULATE_WAIT);
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
        let cache = Cache::new("test", 2, 2, None, TEST_POPULATE_WAIT);
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
}
