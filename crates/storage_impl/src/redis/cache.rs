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
///
/// Enforced by weighing entries with the byte size their caller reports — see [`EntrySize`].
/// Until that weigher existed moka read `max_capacity` as a number of entries, so this figure
/// was multiplied to ~31M entries and never actually bound anything.
pub const DEFAULT_MAX_CAPACITY: u64 = 30;

/// Default entry ceiling for caches bounded by count rather than size.
///
/// A backstop against unbounded growth, not a memory target. What bounds the resident set in
/// normal operation is `time_to_idle`: these caches are keyed per merchant and profile, and an
/// entry unread for its window is dropped, so only recently active merchants stay resident.
/// 10,000 simultaneously active merchant-profile pairs is already a very large deployment, so
/// this should not fire in practice — and if it does, it shows up on
/// `IN_MEMORY_CACHE_EVICTION_COUNT` with `removal_cause=Size`, which is the signal to raise it
/// rather than to let the cache thrash.
pub const DEFAULT_MAX_ENTRIES: u64 = 10_000;

/// Whether a cache's entries arrive with a measurable size.
///
/// Fixed by how a cache is populated rather than by configuration: entries fetched through
/// redis carry the payload size it reported, entries built in-process carry nothing. Either
/// kind can be bounded by entry count; only measured entries can be bounded by megabytes,
/// since unmeasured ones each weigh zero against a size ceiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntrySizing {
    Measured,
    Unmeasured,
}

/// A cache's ceiling, and the unit it is counted in.
///
/// moka bounds a cache by a single number read through its weigher, so the unit is part of the
/// ceiling rather than a setting beside it. Hence one field rather than two: a cache is bounded
/// by size or by count, and "both" is not a state this can be in.
///
/// In configuration:
///
/// ```toml
/// [cache.accounts]
/// max_capacity = { megabytes = 30 }
///
/// [cache.cgraph]
/// max_capacity = { entries = 10000 }
///
/// [cache.config]
/// max_capacity = "unbounded"
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheLimit {
    /// Bounded by the total reported size of its entries.
    Megabytes(u64),
    /// Bounded by how many entries it holds, whatever they weigh.
    Entries(u64),
    /// Bounded only by TTL and time-to-idle.
    Unbounded,
}

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
    /// The cache's ceiling, in whichever unit is chosen — see [`CacheLimit`]. Unset keeps the
    /// cache's own default.
    pub max_capacity: Option<CacheLimit>,
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
    /// Either unit may be chosen for any cache, and [`CacheLimit`] makes it one choice rather
    /// than two competing settings, so there is no "both were set" case to arbitrate.
    ///
    /// The one combination that cannot work is a megabyte ceiling on a cache whose entries
    /// carry no size: each would weigh zero and the ceiling would never be reached. Rather
    /// than accept a setting that silently does nothing, such a cache keeps its default and
    /// says so.
    fn limit(&self, name: &'static str, sizing: EntrySizing, default: CacheLimit) -> CacheLimit {
        match (self.max_capacity, sizing) {
            (Some(CacheLimit::Megabytes(_)), EntrySizing::Unmeasured) => {
                logger::warn!(
                    cache_type = name,
                    "Ignoring a megabyte `max_capacity`: this cache is populated in-process, \
                     so its entries carry no size to weigh and the ceiling would never be \
                     reached. Use `{{ entries = N }}` instead. Falling back to {default:?}"
                );
                default
            }
            (Some(limit), _) => limit,
            (None, _) => default,
        }
    }

    /// Builds the cache this configuration describes.
    ///
    /// `sizing` is fixed by how the cache is populated; `default` is its own ceiling when the
    /// configuration names none.
    fn build(&self, name: &'static str, sizing: EntrySizing, default: CacheLimit) -> Cache {
        Cache::new(
            name,
            self.time_to_live(),
            self.time_to_idle(),
            self.limit(name, sizing, default),
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
        // Both `max_entries` and `max_capacity_in_mb` are accepted for every cache; these are
        // only the defaults when neither is configured. `EntrySizing` records which caches
        // could honour a megabyte ceiling at all — see [`CacheSettings::limit`].
        use EntrySizing::{Measured, Unmeasured};

        const BY_SIZE: CacheLimit = CacheLimit::Megabytes(DEFAULT_MAX_CAPACITY);
        const BY_COUNT: CacheLimit = CacheLimit::Entries(DEFAULT_MAX_ENTRIES);
        const UNBOUNDED: CacheLimit = CacheLimit::Unbounded;

        Self {
            invalidation_channel: config
                .invalidation_channel
                .clone()
                .unwrap_or_else(|| DEFAULT_IMC_INVALIDATION_CHANNEL.to_string()),
            config: config.config.build("CONFIG_CACHE", Measured, UNBOUNDED),
            accounts: config.accounts.build("ACCOUNTS_CACHE", Measured, BY_SIZE),
            mca_list: config.mca_list.build("MCA_LIST_CACHE", Measured, BY_SIZE),
            routing: config.routing.build("ROUTING_CACHE", Unmeasured, BY_COUNT),
            decision_manager: config.decision_manager.build(
                "DECISION_MANAGER_CACHE",
                Measured,
                BY_SIZE,
            ),
            surcharge: config.surcharge.build("SURCHARGE_CACHE", Measured, BY_SIZE),
            cgraph: config.cgraph.build("CGRAPH_CACHE", Unmeasured, BY_COUNT),
            pm_filters_cgraph: config.pm_filters_cgraph.build(
                "PM_FILTERS_CGRAPH_CACHE",
                Unmeasured,
                BY_COUNT,
            ),
            success_based_dynamic_algorithm: config.success_based_dynamic_algorithm.build(
                "SUCCESS_BASED_DYNAMIC_ALGORITHM_CACHE",
                Measured,
                BY_SIZE,
            ),
            elimination_based_dynamic_algorithm: config.elimination_based_dynamic_algorithm.build(
                "ELIMINATION_BASED_DYNAMIC_ALGORITHM_CACHE",
                Measured,
                BY_SIZE,
            ),
            contract_based_dynamic_algorithm: config.contract_based_dynamic_algorithm.build(
                "CONTRACT_BASED_DYNAMIC_ALGORITHM_CACHE",
                Measured,
                BY_SIZE,
            ),
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
    inner: MokaCache<String, WeightedEntry>,
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

/// How much of a cache's byte budget an entry consumes.
///
/// Supplied by the caller rather than measured here. The values these caches hold are
/// arbitrary types behind `Arc<dyn Cacheable>`: their real allocation graph cannot be walked,
/// and `size_of` would see only the shallow struct — for a merchant account, a handful of
/// pointers rather than the strings they point at. Serializing each entry just to measure it
/// would be accurate but would burn CPU on every insert, so instead the size comes from
/// callers that already have it for free.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntrySize {
    /// Serialized size of the value, in bytes.
    ///
    /// Taken from the redis round trip that produced the value: both reading and writing
    /// materialize the payload anyway, so its length costs nothing to know.
    Bytes(usize),
    /// No size available for this entry.
    ///
    /// It contributes nothing to a byte budget, so a cache populated this way cannot be
    /// bounded by one — see the unbounded caches in [`Caches::new`].
    Unmeasured,
}

impl EntrySize {
    /// The weight moka accounts this entry at.
    fn weight(self, key: &str) -> u32 {
        match self {
            // The key is included: moka stores it alongside the value.
            Self::Bytes(bytes) => {
                u32::try_from(bytes.saturating_add(key.len())).unwrap_or(u32::MAX)
            }
            Self::Unmeasured => 0,
        }
    }
}

/// A cached value together with the byte weight moka bounds the cache by.
///
/// The weight travels with the value because moka's weigher is synchronous and only sees
/// `&V`, from which a `dyn Cacheable` cannot be measured.
#[derive(Clone)]
struct WeightedEntry {
    value: Arc<dyn Cacheable>,
    weight: u32,
}

impl Cache {
    /// With given `time_to_live` and `time_to_idle` creates a moka cache.
    ///
    /// `name`        : Cache type name to be used as an attribute in metrics
    /// `time_to_live`: Time in seconds before an object is stored in a caching system before it’s deleted
    /// `time_to_idle`: Time in seconds before a `get` or `insert` operation an object is stored in a caching system before it's deleted
    /// `limit`       : The ceiling, and the unit it is counted in
    pub fn new(
        name: &'static str,
        time_to_live: u64,
        time_to_idle: u64,
        limit: CacheLimit,
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
        // moka reads `max_capacity` through the weigher, so the weigher is what fixes the
        // unit: an entry's reported byte size for a megabyte ceiling, a flat 1 per entry for
        // an entry ceiling. Leaving the weigher off entirely is what made the megabyte figure
        // a no-op before — moka silently counted entries instead.
        let counts_bytes = matches!(limit, CacheLimit::Megabytes(_));
        let mut cache_builder = MokaCache::builder()
            .time_to_live(std::time::Duration::from_secs(time_to_live))
            .time_to_idle(std::time::Duration::from_secs(time_to_idle))
            .weigher(
                move |_key, entry: &WeightedEntry| {
                    if counts_bytes {
                        entry.weight
                    } else {
                        1
                    }
                },
            )
            .eviction_listener(eviction_listener);

        cache_builder = match limit {
            CacheLimit::Megabytes(capacity_in_mb) => {
                cache_builder.max_capacity(capacity_in_mb.saturating_mul(1024 * 1024))
            }
            CacheLimit::Entries(entries) => cache_builder.max_capacity(entries),
            CacheLimit::Unbounded => cache_builder,
        };

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
    pub async fn push<T: Cacheable>(&self, key: CacheKey, val: T, size: EntrySize) {
        let physical_key = in_memory_cache_key(key);
        let weight = size.weight(&physical_key);

        self.inner
            .insert(
                physical_key,
                WeightedEntry {
                    value: Arc::new(val),
                    weight,
                },
            )
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

        let entry = val?;

        // The explicit deref is load-bearing. `Arc<dyn Cacheable>` satisfies the blanket
        // `Cacheable` impl itself, so `entry.value.as_any()` resolves to the `Arc`'s own impl
        // and hands back a `&dyn Any` describing the `Arc` — downcasting which silently turns
        // every read into a miss. Deref first so `as_any` comes from the value inside.
        (*entry.value).as_any().downcast_ref::<T>().cloned()
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
) -> CustomResult<(T, EntrySize), StorageError>
where
    T: serde::Serialize + serde::de::DeserializeOwned + Debug,
    F: FnOnce() -> Fut + Send,
    Fut: futures::Future<Output = CustomResult<T, StorageError>> + Send,
{
    let type_name = std::any::type_name::<T>();
    let key = key.as_ref();
    let redis_val = redis
        .get_and_deserialize_key_with_payload_size::<T>(&key.into(), type_name)
        .await;
    let get_data_set_redis = || async {
        let data = fun().await?;
        let size = match ttl {
            Some(ttl) => {
                redis
                    .serialize_and_set_key_with_expiry(&key.into(), &data, ttl)
                    .await
                    .change_context(StorageError::KVError)?;
                // The expiring setter has no size-reporting variant, and nothing calls this
                // function with a ttl today. Add one alongside
                // `serialize_and_set_key_with_payload_size` if a caller appears that also
                // needs its entries to count against a byte budget.
                EntrySize::Unmeasured
            }
            None => EntrySize::Bytes(
                redis
                    .serialize_and_set_key_with_payload_size(&key.into(), &data)
                    .await
                    .change_context(StorageError::KVError)?,
            ),
        };
        Ok::<_, Report<StorageError>>((data, size))
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
        Ok((val, payload_size)) => Ok((val, EntrySize::Bytes(payload_size))),
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
    // The redis round trip already materialized the payload, so its size is free here and
    // does not have to be recomputed to bound the cache.
    let (val, size) = get_or_populate_redis(redis, key, None, fun).await?;
    cache.push(cache_key, val.clone(), size).await;

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

    /// Either unit may be chosen for any cache, so the resolution is worth pinning as a
    /// table rather than one case at a time.
    #[test]
    fn a_configured_limit_is_honoured_in_whichever_unit_it_names() {
        let by_entries = CacheSettings {
            max_capacity: Some(CacheLimit::Entries(500)),
            ..CacheSettings::default()
        };
        let by_size = CacheSettings {
            max_capacity: Some(CacheLimit::Megabytes(64)),
            ..CacheSettings::default()
        };

        // Entry counts bind either kind of cache.
        for sizing in [EntrySizing::Measured, EntrySizing::Unmeasured] {
            assert_eq!(
                by_entries.limit("TEST", sizing, CacheLimit::Unbounded),
                CacheLimit::Entries(500)
            );
        }

        // A megabyte ceiling binds only where entries report a size.
        assert_eq!(
            by_size.limit("TEST", EntrySizing::Measured, CacheLimit::Unbounded),
            CacheLimit::Megabytes(64)
        );
    }

    #[test]
    fn a_megabyte_limit_on_unmeasured_entries_falls_back_to_the_default() {
        // Honouring it would be worse than ignoring it: every entry weighs zero, so the
        // ceiling would never be reached and the cache would look bounded while growing.
        let settings = CacheSettings {
            max_capacity: Some(CacheLimit::Megabytes(64)),
            ..CacheSettings::default()
        };

        assert_eq!(
            settings.limit("TEST", EntrySizing::Unmeasured, CacheLimit::Entries(10)),
            CacheLimit::Entries(10)
        );
    }

    #[test]
    fn an_unconfigured_cache_keeps_its_own_default() {
        let settings = CacheSettings::default();

        assert_eq!(
            settings.limit("TEST", EntrySizing::Measured, CacheLimit::Megabytes(30)),
            CacheLimit::Megabytes(30)
        );
        assert_eq!(
            settings.limit("TEST", EntrySizing::Unmeasured, CacheLimit::Entries(10_000)),
            CacheLimit::Entries(10_000)
        );
    }

    /// The config shape is the point of [`CacheLimit`] being one field, so pin how it reads.
    #[test]
    fn a_limit_deserializes_in_either_unit_and_never_in_both() {
        let config: CacheConfig = serde_json::from_value(serde_json::json!({
            "accounts": { "max_capacity": { "megabytes": 64 } },
            "cgraph": { "max_capacity": { "entries": 500 } },
            "config": { "max_capacity": "unbounded" },
        }))
        .expect("failed to deserialize cache configuration");

        assert_eq!(
            config.accounts.max_capacity,
            Some(CacheLimit::Megabytes(64))
        );
        assert_eq!(config.cgraph.max_capacity, Some(CacheLimit::Entries(500)));
        assert_eq!(config.config.max_capacity, Some(CacheLimit::Unbounded));
        assert_eq!(config.routing.max_capacity, None);

        // Naming both units is not a shape this can deserialize into.
        serde_json::from_value::<CacheConfig>(serde_json::json!({
            "accounts": { "max_capacity": { "megabytes": 64, "entries": 500 } },
        }))
        .expect_err("a limit in two units at once should not deserialize");
    }

    /// The `[cache]` documentation in `config/*.toml` promises this exact syntax, and TOML
    /// reaches serde through the `config` crate's own value tree rather than directly — so
    /// pin it against real TOML, not just an equivalent JSON shape.
    #[test]
    fn the_documented_toml_syntax_deserializes() {
        let config: CacheConfig = config::Config::builder()
            .add_source(config::File::from_str(
                r#"
                [accounts]
                max_capacity = { megabytes = 64 }

                [cgraph]
                max_capacity = { entries = 500 }

                [config]
                max_capacity = "unbounded"
                "#,
                config::FileFormat::Toml,
            ))
            .build()
            .expect("failed to build cache configuration")
            .try_deserialize()
            .expect("failed to deserialize cache configuration from TOML");

        assert_eq!(
            config.accounts.max_capacity,
            Some(CacheLimit::Megabytes(64))
        );
        assert_eq!(config.cgraph.max_capacity, Some(CacheLimit::Entries(500)));
        assert_eq!(config.config.max_capacity, Some(CacheLimit::Unbounded));
    }

    #[tokio::test]
    async fn an_entry_ceiling_binds_regardless_of_entry_size() {
        // Three entries allowed, eight pushed, and every one is `Unmeasured` — so nothing
        // here has a byte weight that could have bound it.
        let cache = Cache::new("test", 1800, 1800, CacheLimit::Entries(3));
        for index in 0..8 {
            cache
                .push(
                    CacheKey {
                        key: format!("key{index}"),
                        prefix: "prefix".to_string(),
                    },
                    "val".to_string(),
                    EntrySize::Unmeasured,
                )
                .await;
        }
        cache.run_pending_tasks().await;

        assert_eq!(cache.get_entry_count(), 3);
    }

    #[test]
    fn a_measured_entry_weighs_its_payload_plus_its_key() {
        assert_eq!(EntrySize::Bytes(1000).weight("key"), 1000 + 3);
    }

    #[test]
    fn an_unmeasured_entry_weighs_nothing() {
        // Which is why a cache fed unmeasured entries cannot be bounded by bytes, and is
        // left unbounded instead.
        assert_eq!(EntrySize::Unmeasured.weight("key"), 0);
    }

    #[tokio::test]
    async fn a_cache_evicts_once_its_megabyte_budget_is_exceeded() {
        // One megabyte, against eight entries of ~256 KiB: they cannot all be held. Without
        // a weigher moka would read this as a capacity of 1048576 *entries* and keep all
        // eight.
        let cache = Cache::new("test", 1800, 1800, CacheLimit::Megabytes(1));
        for index in 0..8 {
            cache
                .push(
                    CacheKey {
                        key: format!("key{index}"),
                        prefix: "prefix".to_string(),
                    },
                    "x".repeat(256 * 1024),
                    EntrySize::Bytes(256 * 1024),
                )
                .await;
        }
        cache.run_pending_tasks().await;

        let held = cache.get_entry_count();
        assert!(
            (1..8).contains(&held),
            "expected the megabyte budget to bind, but the cache held {held} of 8 entries"
        );
    }

    #[test]
    fn unset_settings_resolve_to_the_compiled_in_defaults() {
        let settings = CacheSettings::default();

        assert_eq!(settings.time_to_live(), DEFAULT_CACHE_TTL);
        assert_eq!(settings.time_to_idle(), DEFAULT_CACHE_TTI);
    }

    #[test]
    fn configured_settings_override_the_defaults() {
        let settings = CacheSettings {
            ttl_in_secs: Some(60),
            tti_in_secs: Some(30),
            max_capacity: Some(CacheLimit::Megabytes(128)),
        };

        assert_eq!(settings.time_to_live(), 60);
        assert_eq!(settings.time_to_idle(), 30);
        assert_eq!(
            settings.limit("TEST", EntrySizing::Measured, CacheLimit::Unbounded),
            CacheLimit::Megabytes(128)
        );
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
            config.accounts.limit(
                "TEST",
                EntrySizing::Measured,
                CacheLimit::Megabytes(DEFAULT_MAX_CAPACITY)
            ),
            CacheLimit::Megabytes(DEFAULT_MAX_CAPACITY)
        );
        assert_eq!(config.routing.time_to_live(), DEFAULT_CACHE_TTL);
        assert_eq!(
            config
                .config
                .limit("TEST", EntrySizing::Measured, CacheLimit::Unbounded),
            CacheLimit::Unbounded
        );
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
        let cache = Cache::new("test", 1800, 1800, CacheLimit::Unbounded);
        cache
            .push(
                CacheKey {
                    key: "key".to_string(),
                    prefix: "prefix".to_string(),
                },
                "val".to_string(),
                EntrySize::Bytes(5),
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
        let cache = Cache::new("test", 2, 2, CacheLimit::Megabytes(0));
        cache
            .push(
                CacheKey {
                    key: "key".to_string(),
                    prefix: "prefix".to_string(),
                },
                "val".to_string(),
                EntrySize::Bytes(5),
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
        let cache = Cache::new("test", 1800, 1800, CacheLimit::Unbounded);
        cache
            .push(
                CacheKey {
                    key: "key".to_string(),
                    prefix: "prefix".to_string(),
                },
                "val".to_string(),
                EntrySize::Bytes(5),
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
        let cache = Cache::new("test", 2, 2, CacheLimit::Unbounded);
        cache
            .push(
                CacheKey {
                    key: "key".to_string(),
                    prefix: "prefix".to_string(),
                },
                "val".to_string(),
                EntrySize::Bytes(5),
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
