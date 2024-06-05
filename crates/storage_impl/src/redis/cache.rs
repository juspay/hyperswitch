use std::{any::Any, borrow::Cow, fmt::Debug, sync::Arc};

use common_utils::{
    errors::{self, CustomResult},
    ext_traits::AsyncExt,
};
use dyn_clone::DynClone;
use error_stack::{Report, ResultExt};
use moka::future::Cache as MokaCache;
use once_cell::sync::Lazy;
use redis_interface::{errors::RedisError, RedisConnectionPool, RedisValue};
use router_env::tracing::{self, instrument};

use crate::{
    errors::StorageError,
    redis::{PubSubInterface, RedisConnInterface},
};

/// Redis channel name used for publishing invalidation messages
pub const PUB_SUB_CHANNEL: &str = "hyperswitch_invalidate";

/// Prefix for config cache key
const CONFIG_CACHE_PREFIX: &str = "config";

/// Prefix for accounts cache key
const ACCOUNTS_CACHE_PREFIX: &str = "accounts";

/// Prefix for routing cache key
const ROUTING_CACHE_PREFIX: &str = "routing";

/// Prefix for cgraph cache key
const CGRAPH_CACHE_PREFIX: &str = "cgraph";

/// Prefix for all kinds of cache key
const ALL_CACHE_PREFIX: &str = "all_cache_kind";

/// Time to live 30 mins
const CACHE_TTL: u64 = 30 * 60;

/// Time to idle 10 mins
const CACHE_TTI: u64 = 10 * 60;

/// Max Capacity of Cache in MB
const MAX_CAPACITY: u64 = 30;

/// Config Cache with time_to_live as 30 mins and time_to_idle as 10 mins.
pub static CONFIG_CACHE: Lazy<Cache> = Lazy::new(|| Cache::new(CACHE_TTL, CACHE_TTI, None));

/// Accounts cache with time_to_live as 30 mins and size limit
pub static ACCOUNTS_CACHE: Lazy<Cache> =
    Lazy::new(|| Cache::new(CACHE_TTL, CACHE_TTI, Some(MAX_CAPACITY)));

/// Routing Cache
pub static ROUTING_CACHE: Lazy<Cache> =
    Lazy::new(|| Cache::new(CACHE_TTL, CACHE_TTI, Some(MAX_CAPACITY)));

/// CGraph Cache
pub static CGRAPH_CACHE: Lazy<Cache> =
    Lazy::new(|| Cache::new(CACHE_TTL, CACHE_TTI, Some(MAX_CAPACITY)));

/// Trait which defines the behaviour of types that's gonna be stored in Cache
pub trait Cacheable: Any + Send + Sync + DynClone {
    fn as_any(&self) -> &dyn Any;
}

pub enum CacheKind<'a> {
    Config(Cow<'a, str>),
    Accounts(Cow<'a, str>),
    Routing(Cow<'a, str>),
    CGraph(Cow<'a, str>),
    All(Cow<'a, str>),
}

impl<'a> From<CacheKind<'a>> for RedisValue {
    fn from(kind: CacheKind<'a>) -> Self {
        let value = match kind {
            CacheKind::Config(s) => format!("{CONFIG_CACHE_PREFIX},{s}"),
            CacheKind::Accounts(s) => format!("{ACCOUNTS_CACHE_PREFIX},{s}"),
            CacheKind::Routing(s) => format!("{ROUTING_CACHE_PREFIX},{s}"),
            CacheKind::CGraph(s) => format!("{CGRAPH_CACHE_PREFIX},{s}"),
            CacheKind::All(s) => format!("{ALL_CACHE_PREFIX},{s}"),
        };
        Self::from_string(value)
    }
}

impl<'a> TryFrom<RedisValue> for CacheKind<'a> {
    type Error = Report<errors::ValidationError>;
    fn try_from(kind: RedisValue) -> Result<Self, Self::Error> {
        let validation_err = errors::ValidationError::InvalidValue {
            message: "Invalid publish key provided in pubsub".into(),
        };
        let kind = kind.as_string().ok_or(validation_err.clone())?;
        let split = kind.split_once(',').ok_or(validation_err.clone())?;
        match split.0 {
            ACCOUNTS_CACHE_PREFIX => Ok(Self::Accounts(Cow::Owned(split.1.to_string()))),
            CONFIG_CACHE_PREFIX => Ok(Self::Config(Cow::Owned(split.1.to_string()))),
            ROUTING_CACHE_PREFIX => Ok(Self::Routing(Cow::Owned(split.1.to_string()))),
            CGRAPH_CACHE_PREFIX => Ok(Self::CGraph(Cow::Owned(split.1.to_string()))),
            ALL_CACHE_PREFIX => Ok(Self::All(Cow::Owned(split.1.to_string()))),
            _ => Err(validation_err.into()),
        }
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
    /// `time_to_live`: Time in seconds before an object is stored in a caching system before it’s deleted
    /// `time_to_idle`: Time in seconds before a `get` or `insert` operation an object is stored in a caching system before it's deleted
    /// `max_capacity`: Max size in MB's that the cache can hold
    pub fn new(time_to_live: u64, time_to_idle: u64, max_capacity: Option<u64>) -> Self {
        let mut cache_builder = MokaCache::builder()
            .time_to_live(std::time::Duration::from_secs(time_to_live))
            .time_to_idle(std::time::Duration::from_secs(time_to_idle));

        if let Some(capacity) = max_capacity {
            cache_builder = cache_builder.max_capacity(capacity * 1024 * 1024);
        }

        Self {
            inner: cache_builder.build(),
        }
    }

    pub async fn push<T: Cacheable>(&self, key: CacheKey, val: T) {
        self.inner.insert(key.into(), Arc::new(val)).await;
    }

    pub async fn get_val<T: Clone + Cacheable>(&self, key: CacheKey) -> Option<T> {
        let val = self.inner.get::<String>(&key.into()).await?;
        (*val).as_any().downcast_ref::<T>().cloned()
    }

    /// Check if a key exists in cache
    pub async fn exists(&self, key: CacheKey) -> bool {
        self.inner.contains_key::<String>(&key.into())
    }

    pub async fn remove(&self, key: CacheKey) {
        self.inner.invalidate::<String>(&key.into()).await;
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
    let redis_val = redis.get_and_deserialize_key::<T>(key, type_name).await;
    let get_data_set_redis = || async {
        let data = fun().await?;
        redis
            .serialize_and_set_key(key, &data)
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
pub async fn redact_cache<T, F, Fut>(
    store: &(dyn RedisConnInterface + Send + Sync),
    key: &'static str,
    fun: F,
    in_memory: Option<&Cache>,
) -> CustomResult<T, StorageError>
where
    F: FnOnce() -> Fut + Send,
    Fut: futures::Future<Output = CustomResult<T, StorageError>> + Send,
{
    let data = fun().await?;

    let redis_conn = store
        .get_redis_conn()
        .change_context(StorageError::RedisError(
            RedisError::RedisConnectionError.into(),
        ))
        .attach_printable("Failed to get redis connection")?;
    let tenant_key = CacheKey {
        key: key.to_string(),
        prefix: redis_conn.key_prefix.clone(),
    };
    in_memory.async_map(|cache| cache.remove(tenant_key)).await;

    redis_conn
        .delete_key(key)
        .await
        .change_context(StorageError::KVError)?;
    Ok(data)
}

#[instrument(skip_all)]
pub async fn publish_into_redact_channel<'a, K: IntoIterator<Item = CacheKind<'a>> + Send>(
    store: &(dyn RedisConnInterface + Send + Sync),
    keys: K,
) -> CustomResult<usize, StorageError> {
    let redis_conn = store
        .get_redis_conn()
        .change_context(StorageError::RedisError(
            RedisError::RedisConnectionError.into(),
        ))
        .attach_printable("Failed to get redis connection")?;

    let futures = keys.into_iter().map(|key| async {
        redis_conn
            .clone()
            .publish(PUB_SUB_CHANNEL, key)
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
    publish_into_redact_channel(store, [key]).await?;
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
    K: IntoIterator<Item = CacheKind<'a>> + Send,
{
    let data = fun().await?;
    publish_into_redact_channel(store, keys).await?;
    Ok(data)
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    #[tokio::test]
    async fn construct_and_get_cache() {
        let cache = Cache::new(1800, 1800, None);
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
        let cache = Cache::new(2, 2, Some(0));
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
        let cache = Cache::new(1800, 1800, None);
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
        let cache = Cache::new(2, 2, None);
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
