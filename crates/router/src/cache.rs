use std::any::Any;

use dyn_clone::DynClone;
use moka::future::Cache as MokaCache;

/// Trait which defines the behaviour of types that's gonna be stored in Cache
pub trait Cacheable: Any + Send + Sync + DynClone {
    fn as_any(&self) -> &dyn Any;
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
    inner: MokaCache<String, Box<dyn Cacheable>>,
}

impl std::ops::Deref for Cache {
    type Target = MokaCache<String, Box<dyn Cacheable>>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Cache {
    /// With given `time_to_live` and `time_to_idle` creates a moka cache.
    ///
    /// `time_to_live`: Time in seconds before an object is stored in a caching system before it’s deleted
    /// `time_to_idle`: Time in seconds before a `get` or `insert` operation an object is stored in a caching system before it's deleted
    pub fn new(time_to_live: u64, time_to_idle: u64) -> Self {
        Self {
            inner: MokaCache::builder()
                .eviction_listener_with_queued_delivery_mode(|_, _, _| {})
                .time_to_live(std::time::Duration::from_secs(time_to_live))
                .time_to_idle(std::time::Duration::from_secs(time_to_idle))
                .build(),
        }
    }

    pub fn get_val<T: Clone + Any>(&self, key: &str) -> Option<T> {
        let val = self.get(key)?;
        (*val).as_any().downcast_ref::<T>().cloned()
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    #[tokio::test]
    async fn construct_and_get_cache() {
        let cache = Cache::new(1800, 1800);
        cache
            .insert("key".to_string(), Box::new("val".to_string()))
            .await;
        assert_eq!(cache.get_val::<String>("key"), Some(String::from("val")));
    }
}
