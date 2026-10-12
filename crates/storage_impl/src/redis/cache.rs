use std::{any::Any, borrow::Cow, fmt::Debug, num::NonZeroU64, sync::Arc, time::Duration};

use common_utils::{
    errors::{self, CustomResult},
    ext_traits::ByteSliceExt,
};
use dyn_clone::DynClone;
use error_stack::{Report, ResultExt};
use moka::{
    future::Cache as MokaCache,
    ops::compute::{CompResult, Op},
};
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

    fn record_invariant_violation(&self) {
        metrics::IN_MEMORY_CACHE_INVARIANT_VIOLATION
            .add(1, router_env::metric_attributes!(("cache_type", self.name)));
        logger::error!(
            cache_type = self.name,
            "and_try_compute_with returned a CompResult variant get_or_populate_in_memory never asks for"
        );
    }

    /// Reads `key`, and on a miss populates it by awaiting `populate`.
    ///
    /// Concurrent callers for one key are serialized by moka's per-key lock
    /// (`and_try_compute_with`): each caller's own `populate` runs in turn against the
    /// then-current state, so a failure or a timed-out attempt only ever affects the caller it
    /// happened to, never whoever's queued behind it. A single attempt is capped at
    /// `populate_timeout`; since a caller's turn ends the moment its future resolves, this
    /// also bounds how long anyone queued behind it can be made to wait. Every attempt is
    /// capped — there is no value of `populate_timeout` that opts out of the cap.
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
        // Fast path: a hit never touches moka's entry API or its per-key lock at all.
        if let Some(val) = self.get_val::<T>(key.clone()).await {
            return Ok(val);
        }

        let moka_key = in_memory_cache_key(key.clone());
        let populate_timeout = self.populate_timeout;

        // moka's `Op::Put` does the real insert; `record_populate_insert` below just mirrors
        // `push`'s deja boundary so the write is still recorded.
        let outcome = self
            .inner
            .entry(moka_key)
            .and_try_compute_with(move |maybe_entry| async move {
                if maybe_entry.is_some() {
                    return Ok(Op::Nop);
                }

                match tokio::time::timeout(populate_timeout, populate).await {
                    Ok(Ok(val)) => {
                        let val: Arc<dyn Cacheable> = Arc::new(val);
                        Ok(Op::Put(val))
                    }
                    Ok(Err(e)) => Err(Some(e)),
                    Err(_elapsed) => Err(None),
                }
            })
            .await;

        match outcome {
            // A same-typed entry is expected here; a downcast failure means some other
            // caller used this key for a different `T`.
            Ok(CompResult::Unchanged(entry)) => {
                self.record_population_avoided();
                downcast_cacheable::<T>(entry.into_value()).ok_or_else(|| {
                    self.record_type_mismatch();
                    StorageError::CacheTypeMismatch.into()
                })
            }
            // `ReplacedWith` can't actually happen here (we never `Op::Put` over an existing
            // entry), but it carries a value just like `Inserted` does.
            Ok(CompResult::Inserted(entry) | CompResult::ReplacedWith(entry)) => {
                self.record_populate_insert(key.clone()).await;
                downcast_cacheable::<T>(entry.into_value()).ok_or_else(|| {
                    self.record_type_mismatch();
                    StorageError::CacheTypeMismatch.into()
                })
            }
            // Can't happen either: we never ask for `Op::Remove`, and `Op::Nop` only follows
            // an already-present entry.
            Ok(CompResult::Removed(_) | CompResult::StillNone(_)) => {
                self.record_invariant_violation();
                Err(StorageError::CacheInvariantViolation.into())
            }
            Err(Some(e)) => Err(e),
            // Our own cap elapsed, not the backend — there's no real `E` to report. Our turn
            // already ended, so anyone queued behind us has already moved on.
            Err(None) => {
                self.record_populate_timeout();
                Err(StorageError::CachePopulateTimedOut.into())
            }
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

    /// Records `push`'s exact `in_memory_push` boundary for a write that moka's own
    /// `and_try_compute_with` (`Op::Put`) performed directly. deja's call-site identity hashes
    /// the `boundary`/`operation` literals, not this function's body, so this is
    /// indistinguishable from a real `push` call to its recording.
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
pub async fn get_or_populate_redis<T, Fut>(
    redis: &RedisConnectionWithContext,
    key: impl AsRef<str>,
    ttl: Option<i64>,
    populate: Fut,
) -> CustomResult<T, StorageError>
where
    T: serde::Serialize + serde::de::DeserializeOwned + Debug,
    Fut: futures::Future<Output = CustomResult<T, StorageError>> + Send,
{
    let type_name = std::any::type_name::<T>();
    let key = key.as_ref();
    let redis_val = redis
        .get_and_deserialize_key::<T>(&key.into(), type_name)
        .await;
    match redis_val {
        Err(err) => match err.current_context() {
            RedisError::NotFound | RedisError::JsonDeserializationFailed => {
                let data = populate.await?;
                match ttl {
                    Some(ttl) => {
                        redis
                            .serialize_and_set_key_with_expiry(&key.into(), &data, ttl)
                            .await
                    }
                    None => redis.serialize_and_set_key(&key.into(), &data).await,
                }
                .change_context(StorageError::KVError)?;
                Ok(data)
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
    let cache = store.caches().get(cache);
    let cache_key = CacheKey {
        key: key.to_string(),
        prefix: store.cache_key_prefix().to_string(),
    };
    // Redis is the layer behind this one, so it is held past this cache's own lifetime.
    let redis_ttl = cache.redis_ttl_for(key);

    // The redis connection is acquired only when this caller is the one populating, so an
    // in-memory hit answers during a redis outage rather than erroring, and concurrent
    // misses for one key cost a single redis round trip between them.
    cache
        .get_or_populate_in_memory(cache_key, async {
            let redis = store
                .get_redis_conn()
                .change_context(StorageError::RedisError(
                    RedisError::RedisConnectionError.into(),
                ))
                .attach_printable("Failed to get redis connection")?;
            get_or_populate_redis(&redis, key, redis_ttl, fun).await
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
            .collect::<std::collections::HashSet<_>>();

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
}
