use common_utils::ext_traits::AsyncExt;
use error_stack::ResultExt;

use super::MockDb;
use super::Store;
use crate::{
    cache::{self, Cacheable},
    cache::CONFIG_CACHE,
    consts,
    core::errors::{self, CustomResult},
    services::PubSubInterface,
};

#[async_trait::async_trait]
pub trait CacheInterface {
    async fn invalidate(&self, key: &str) -> Result<(), String>;
}

#[async_trait::async_trait]
impl CacheInterface for Store {
    async fn invalidate(&self, key: &str) -> Result<(), String> {
        CONFIG_CACHE.remove(key).await;
        Ok(())
    }
}

#[async_trait::async_trait]
impl CacheInterface for MockDb {
    async fn invalidate(&self, _key: &str) -> Result<(), String> {
        Ok(())
    }
}

pub async fn get_or_populate_redis<T, F, Fut>(
    store: &Store,
    key: &str,
    fun: F,
) -> CustomResult<T, errors::StorageError>
where
    T: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
    F: FnOnce() -> Fut + Send,
    Fut: futures::Future<Output = CustomResult<T, errors::StorageError>> + Send,
{
    let type_name = std::any::type_name::<T>();
    let redis = &store
        .redis_conn()
        .map_err(Into::<errors::StorageError>::into)?;
    let redis_val = redis.get_and_deserialize_key::<T>(key, type_name).await;
    let get_data_set_redis = || async {
        let data = fun().await?;
        redis
            .serialize_and_set_key(key, &data)
            .await
            .change_context(errors::StorageError::KVError)?;
        Ok::<_, error_stack::Report<errors::StorageError>>(data)
    };
    match redis_val {
        Err(err) => match err.current_context() {
            errors::RedisError::NotFound | errors::RedisError::JsonDeserializationFailed => {
                get_data_set_redis().await
            }
            _ => Err(err
                .change_context(errors::StorageError::KVError)
                .attach_printable(format!("Error while fetching cache for {type_name}"))),
        },
        Ok(val) => Ok(val),
    }
}

pub async fn get_or_populate_in_memory<T, F, Fut>(
    store: &Store,
    key: &str,
    fun: F,
    cache: &cache::Cache,
) -> CustomResult<T, errors::StorageError>
where
    T: Cacheable + serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug + Clone,
    F: FnOnce() -> Fut + Send,
    Fut: futures::Future<Output = CustomResult<T, errors::StorageError>> + Send,
{
    let cache_val = cache.get_val::<T>(key);
    if let Some(val) = cache_val {
        Ok(val)
    } else {
        let val = get_or_populate_redis(store, key, fun).await?;
        cache.push(key.to_string(), val.clone()).await;
        Ok(val)
    }
}

pub async fn redact_cache<T, F, Fut>(
    store: &Store,
    key: &str,
    fun: F,
    in_memory: Option<&cache::Cache>,
) -> CustomResult<T, errors::StorageError>
where
    F: FnOnce() -> Fut + Send,
    Fut: futures::Future<Output = CustomResult<T, errors::StorageError>> + Send,
{
    let data = fun().await?;
    in_memory.async_map(|cache| cache.invalidate(key)).await;
    store
        .redis_conn()
        .map_err(Into::<errors::StorageError>::into)?
        .delete_key(key)
        .await
        .change_context(errors::StorageError::KVError)?;
    Ok(data)
}

pub async fn publish_and_redact<T, F, Fut>(
    store: &Store,
    key: &str,
    fun: F,
) -> CustomResult<T, errors::StorageError>
where
    F: FnOnce() -> Fut + Send,
    Fut: futures::Future<Output = CustomResult<T, errors::StorageError>> + Send,
{
    let data = fun().await?;
    store
        .redis_conn()
        .map_err(Into::<errors::StorageError>::into)?
        .publish(consts::PUB_SUB_CHANNEL, key)
        .await
        .change_context(errors::StorageError::KVError)?;
    Ok(data)
}
