use masking::Secret;
use redis::CacheStore;
pub mod config;
pub mod diesel;
pub mod redis;

pub use crate::diesel::store::DatabaseStore;

pub struct RouterStore<T: DatabaseStore> {
    db_store: T,
    cache_store: CacheStore,
    master_encryption_key: Secret<Vec<u8>>,
}

impl<T: DatabaseStore> RouterStore<T> {
    pub async fn new(
        db_conf: T::Config,
        cache_conf: &redis_interface::RedisSettings,
        encryption_key: Secret<Vec<u8>>,
    ) -> Self {
        // TODO: create an error enum and return proper error here
        let db_store = T::new(db_conf, false).await;
        let cache_store = CacheStore::new(cache_conf)
            .await
            .expect("Failed to create cache store");
        Self {
            db_store,
            cache_store,
            master_encryption_key: encryption_key,
        }
    }
    pub async fn test_store(
        db_conf: T::Config,
        cache_conf: &redis_interface::RedisSettings,
        encryption_key: Secret<Vec<u8>>,
    ) -> Self {
        // TODO: create an error enum and return proper error here
        let db_store = T::new(db_conf, true).await;
        let cache_store = CacheStore::new(cache_conf)
            .await
            .expect("Failed to create cache store");
        Self {
            db_store,
            cache_store,
            master_encryption_key: encryption_key,
        }
    }
}

pub struct KVRouterStore<T: DatabaseStore> {
    router_store: RouterStore<T>,
    drainer_stream_name: String,
    drainer_num_partitions: u8,
}

impl<T: DatabaseStore> KVRouterStore<T> {
    pub fn from_store(
        store: RouterStore<T>,
        drainer_stream_name: String,
        drainer_num_partitions: u8,
    ) -> Self {
        Self {
            router_store: store,
            drainer_stream_name,
            drainer_num_partitions,
        }
    }
}
