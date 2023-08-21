use std::{any::Any, borrow::Cow, sync::Arc};

use dyn_clone::DynClone;
use error_stack::Report;
use moka::future::Cache as MokaCache;
use once_cell::sync::Lazy;
use redis_interface::RedisValue;

use crate::core::errors;

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
            .eviction_listener_with_queued_delivery_mode(|_, _, _| {})
            .time_to_live(std::time::Duration::from_secs(time_to_live))
            .time_to_idle(std::time::Duration::from_secs(time_to_idle));

        if let Some(capacity) = max_capacity {
            cache_builder = cache_builder.max_capacity(capacity * 1024 * 1024);
        }

        Self {
            inner: cache_builder.build(),
        }
    }

    pub async fn push<T: Cacheable>(&self, key: String, val: T) {
        self.insert(key, Arc::new(val)).await;
    }

    pub fn get_val<T: Clone + Cacheable>(&self, key: &str) -> Option<T> {
        let val = self.get(key)?;
        (*val).as_any().downcast_ref::<T>().cloned()
    }

    pub async fn remove(&self, key: &str) {
        self.invalidate(key).await;
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    #[tokio::test]
    async fn construct_and_get_cache() {
        let cache = Cache::new(1800, 1800, None);
        cache.push("key".to_string(), "val".to_string()).await;
        assert_eq!(cache.get_val::<String>("key"), Some(String::from("val")));
    }

    #[tokio::test]
    async fn eviction_on_size_test() {
        let cache = Cache::new(2, 2, Some(0));
        cache.push("key".to_string(), "val".to_string()).await;
        assert_eq!(cache.get_val::<String>("key"), None);
    }

    #[tokio::test]
    async fn invalidate_cache_for_key() {
        let cache = Cache::new(1800, 1800, None);
        cache.push("key".to_string(), "val".to_string()).await;

        cache.remove("key").await;

        assert_eq!(cache.get_val::<String>("key"), None);
    }

    #[tokio::test]
    async fn eviction_on_time_test() {
        let cache = Cache::new(2, 2, None);
        cache.push("key".to_string(), "val".to_string()).await;
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        assert_eq!(cache.get_val::<String>("key"), None);
    }
}
