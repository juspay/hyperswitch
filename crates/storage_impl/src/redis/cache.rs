use std::{any::Any, borrow::Cow, sync::Arc};

use common_utils::{
    errors::{self, CustomResult},
    ext_traits::AsyncExt,
};
use data_models::errors::StorageError;
use dyn_clone::DynClone;
use error_stack::{Report, ResultExt};
use moka::future::Cache as MokaCache;
use once_cell::sync::Lazy;
use redis_interface::{errors::RedisError, RedisValue};

use super::{kv_store::RedisConnInterface, pub_sub::PubSubInterface};

pub(crate) const PUB_SUB_CHANNEL: &str = "hyperswitch_invalidate";

/// Prefix for config cache key
const CONFIG_CACHE_PREFIX: &str = "config";

/// Prefix for accounts cache key
const ACCOUNTS_CACHE_PREFIX: &str = "accounts";

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

/// Trait which defines the behaviour of types that's gonna be stored in Cache
pub trait Cacheable: Any + Send + Sync + DynClone {
    fn as_any(&self) -> &dyn Any;
}

pub enum CacheKind<'a> {
    Config(Cow<'a, str>),
    Accounts(Cow<'a, str>),
    All(Cow<'a, str>),
}

impl<'a> From<CacheKind<'a>> for RedisValue {
        /// Converts a CacheKind enum variant into a corresponding string value and creates a new instance of Self using the string value.
    fn from(kind: CacheKind<'a>) -> Self {
        let value = match kind {
            CacheKind::Config(s) => format!("{CONFIG_CACHE_PREFIX},{s}"),
            CacheKind::Accounts(s) => format!("{ACCOUNTS_CACHE_PREFIX},{s}"),
            CacheKind::All(s) => format!("{ALL_CACHE_PREFIX},{s}"),
        };
        Self::from_string(value)
    }
}

impl<'a> TryFrom<RedisValue> for CacheKind<'a> {
    type Error = Report<errors::ValidationError>;
        /// Tries to convert a RedisValue into a specific enum variant, returning a Result.
    fn try_from(kind: RedisValue) -> Result<Self, Self::Error> {
        let validation_err = errors::ValidationError::InvalidValue {
            message: "Invalid publish key provided in pubsub".into(),
        };
        let kind = kind.as_string().ok_or(validation_err.clone())?;
        let split = kind.split_once(',').ok_or(validation_err.clone())?;
        match split.0 {
            ACCOUNTS_CACHE_PREFIX => Ok(Self::Accounts(Cow::Owned(split.1.to_string()))),
            CONFIG_CACHE_PREFIX => Ok(Self::Config(Cow::Owned(split.1.to_string()))),
            ALL_CACHE_PREFIX => Ok(Self::All(Cow::Owned(split.1.to_string()))),
            _ => Err(validation_err.into()),
        }
    }
}

impl<T> Cacheable for T
where
    T: Any + Clone + Send + Sync,
{
        /// Returns a reference to the trait object `dyn Any` for the current object.
    fn as_any(&self) -> &dyn Any {
        self
    }
}

dyn_clone::clone_trait_object!(Cacheable);

pub struct Cache {
    inner: MokaCache<String, Arc<dyn Cacheable>>,
}

impl std::ops::Deref for Cache {
    type Target = MokaCache<String, Arc<dyn Cacheable>>;
        /// This method returns a reference to the inner data of the current instance.
    fn deref(&self) -> &Self::Target {
        &self.inner
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

        /// Asynchronously pushes a value into the cache with the specified key. The value must implement the Cacheable trait.
    pub async fn push<T: Cacheable>(&self, key: String, val: T) {
        self.insert(key, Arc::new(val)).await;
    }

        /// Asynchronously retrieves a value from the cache by its key and attempts to downcast it to the specified type.
    /// If the value is found in the cache and can be downcast to the specified type, it is cloned and returned, otherwise returns None.
    pub async fn get_val<T: Clone + Cacheable>(&self, key: &str) -> Option<T> {
        let val = self.get(key).await?;
        (*val).as_any().downcast_ref::<T>().cloned()
    }

        /// Asynchronously removes the value associated with the specified key from the cache.
    pub async fn remove(&self, key: &str) {
        self.invalidate(key).await;
    }
}

/// Asynchronously gets the value associated with the specified key from Redis if it exists, or populates the value by executing the provided function and storing the result in Redis. Returns a `CustomResult` containing the retrieved or populated value, or a `StorageError` if an error occurs during the process.
pub async fn get_or_populate_redis<T, F, Fut>(
    store: &(dyn RedisConnInterface + Send + Sync),
    key: impl AsRef<str>,
    fun: F,
) -> CustomResult<T, StorageError>
where
    T: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
    F: FnOnce() -> Fut + Send,
    Fut: futures::Future<Output = CustomResult<T, StorageError>> + Send,
{
    let type_name = std::any::type_name::<T>();
    let key = key.as_ref();
    let redis = &store
        .get_redis_conn()
        .map_err(|er| {
            let error = format!("{}", er);
            er.change_context(StorageError::RedisError(error))
        })
        .attach_printable("Failed to get redis connection")?;
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

/// Asynchronously gets the value from the in-memory cache using the provided key. If the value is not found in the cache, it populates the value by fetching it from Redis using the provided function and then stores it in the cache. Returns the value either from the cache or from the Redis store.
pub async fn get_or_populate_in_memory<T, F, Fut>(
    store: &(dyn RedisConnInterface + Send + Sync),
    key: &str,
    fun: F,
    cache: &Cache,
) -> CustomResult<T, StorageError>
where
    T: Cacheable + serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug + Clone,
    F: FnOnce() -> Fut + Send,
    Fut: futures::Future<Output = CustomResult<T, StorageError>> + Send,
{
    let cache_val = cache.get_val::<T>(key).await;
    if let Some(val) = cache_val {
        Ok(val)
    } else {
        let val = get_or_populate_redis(store, key, fun).await?;
        cache.push(key.to_string(), val.clone()).await;
        Ok(val)
    }
}

/// Asynchronously redacts a value from the cache using the provided key and function, and then deletes the corresponding data from the Redis store. 
/// 
/// # Arguments
/// * `store` - A reference to a trait object implementing the RedisConnInterface.
/// * `key` - The key used to access the value in the cache and Redis store.
/// * `fun` - A closure that returns a future with the result to redact from the cache and store.
/// * `in_memory` - An optional reference to the in-memory cache.
/// 
/// # Returns
/// A custom result containing the redacted data or a storage error.
pub async fn redact_cache<T, F, Fut>(
    store: &dyn RedisConnInterface,
    key: &str,
    fun: F,
    in_memory: Option<&Cache>,
) -> CustomResult<T, StorageError>
where
    F: FnOnce() -> Fut + Send,
    Fut: futures::Future<Output = CustomResult<T, StorageError>> + Send,
{
    let data = fun().await?;
    in_memory.async_map(|cache| cache.invalidate(key)).await;

    let redis_conn = store
        .get_redis_conn()
        .map_err(|er| {
            let error = format!("{}", er);
            er.change_context(StorageError::RedisError(error))
        })
        .attach_printable("Failed to get redis connection")?;

    redis_conn
        .delete_key(key)
        .await
        .change_context(StorageError::KVError)?;
    Ok(data)
}

/// Asynchronously publishes the given `key` into the redact channel using the provided `store`.
pub async fn publish_into_redact_channel<'a>(
    store: &dyn RedisConnInterface,
    key: CacheKind<'a>,
) -> CustomResult<usize, StorageError> {
    let redis_conn = store
        .get_redis_conn()
        .map_err(|er| {
            let error = format!("{}", er);
            er.change_context(StorageError::RedisError(error))
        })
        .attach_printable("Failed to get redis connection")?;

    redis_conn
        .publish(PUB_SUB_CHANNEL, key)
        .await
        .change_context(StorageError::KVError)
}

/// Asynchronously publishes data into a Redis channel and then redacts the data from the cache
pub async fn publish_and_redact<'a, T, F, Fut>(
    store: &dyn RedisConnInterface,
    key: CacheKind<'a>,
    fun: F,
) -> CustomResult<T, StorageError>
where
    F: FnOnce() -> Fut + Send,
    Fut: futures::Future<Output = CustomResult<T, StorageError>> + Send,
{
    let data = fun().await?;
    publish_into_redact_channel(store, key).await?;
    Ok(data)
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    #[tokio::test]
        /// Asynchronously constructs a new cache with a specified expiration time and then adds a key-value pair to the cache. It then checks if the value can be retrieved from the cache and asserts that the retrieved value matches the expected value.
    async fn construct_and_get_cache() {
        let cache = Cache::new(1800, 1800, None);
        cache.push("key".to_string(), "val".to_string()).await;
        assert_eq!(
            cache.get_val::<String>("key").await,
            Some(String::from("val"))
        );
    }

    #[tokio::test]
        /// This method is a test function to verify the eviction behavior of the cache based on size. It creates a new cache with a capacity of 2 items and a maximum size of 2 bytes, and initializes it with a specified eviction policy. It then pushes a key-value pair into the cache and asserts that the value is not present in the cache after the eviction policy has been applied.
    async fn eviction_on_size_test() {
        let cache = Cache::new(2, 2, Some(0));
        cache.push("key".to_string(), "val".to_string()).await;
        assert_eq!(cache.get_val::<String>("key").await, None);
    }

    #[tokio::test]
        /// Asynchronously invalidates the cache for a specific key by adding a new value, removing the key, and then asserting that the value for the key is None.
    async fn invalidate_cache_for_key() {
        let cache = Cache::new(1800, 1800, None);
        cache.push("key".to_string(), "val".to_string()).await;

        cache.remove("key").await;

        assert_eq!(cache.get_val::<String>("key").await, None);
    }

    #[tokio::test]
        /// Executes a test to ensure that the cache eviction occurs on time.
    ///
    /// This method initializes a new cache with a maximum capacity of 2 items and a time-to-live of 2 seconds. It then pushes a key-value pair into the cache and waits for 3 seconds using the `tokio::time::sleep` function. After the wait, it checks that the value associated with the key has been evicted from the cache by asserting that the `get_val` method returns `None`.
    ///
    async fn eviction_on_time_test() {
        let cache = Cache::new(2, 2, None);
        cache.push("key".to_string(), "val".to_string()).await;
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        assert_eq!(cache.get_val::<String>("key").await, None);
    }
}
