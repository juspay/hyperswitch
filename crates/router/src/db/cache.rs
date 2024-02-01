use common_utils::ext_traits::AsyncExt;
use error_stack::ResultExt;
use redis_interface::errors::RedisError;
use storage_impl::redis::{
    cache::{Cache, CacheKind, Cacheable},
    pub_sub::PubSubInterface,
};

use super::StorageInterface;
use crate::{
    consts,
    core::errors::{self, CustomResult},
};

/// Asynchronously retrieves a value from Redis using the provided key. If the value is not found in Redis or if deserialization fails, the provided function `fun` is called to fetch the value and store it in Redis before returning it. 
pub async fn get_or_populate_redis<T, F, Fut>(
    store: &dyn StorageInterface,
    key: impl AsRef<str>,
    fun: F,
) -> CustomResult<T, errors::StorageError>
where
    T: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
    F: FnOnce() -> Fut + Send,
    Fut: futures::Future<Output = CustomResult<T, errors::StorageError>> + Send,
{
    let type_name = std::any::type_name::<T>();
    let key = key.as_ref();
    let redis = &store
        .get_redis_conn()
        .change_context(errors::StorageError::RedisError(
            RedisError::RedisConnectionError.into(),
        ))
        .attach_printable("Failed to get redis connection")?;
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

/// Asynchronously retrieves a value from an in-memory cache, or populates the cache by calling a provided function if the value is not found. This method takes a store, key, function, and cache as parameters, where the store is the storage interface, key is the key to retrieve from the cache, fun is the function to populate the cache if the value is not found, and cache is the in-memory cache. The method returns a CustomResult containing the retrieved or populated value, or an error if the operation fails.
pub async fn get_or_populate_in_memory<T, F, Fut>(
    store: &dyn StorageInterface,
    key: &str,
    fun: F,
    cache: &Cache,
) -> CustomResult<T, errors::StorageError>
where
    T: Cacheable + serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug + Clone,
    F: FnOnce() -> Fut + Send,
    Fut: futures::Future<Output = CustomResult<T, errors::StorageError>> + Send,
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

/// Asynchronously redacts a value from the cache using the provided key and a function to retrieve the data if not found in the cache. The redacted value is also deleted from the Redis store.
/// 
/// # Arguments
/// * `store` - A reference to a storage interface
/// * `key` - The key used to retrieve the value from the cache and Redis store
/// * `fun` - A closure that returns a future containing the data to be redacted
/// * `in_memory` - An optional reference to a Cache instance for in-memory caching
/// 
/// # Returns
/// A custom result containing the redacted data or a storage error
pub async fn redact_cache<T, F, Fut>(
    store: &dyn StorageInterface,
    key: &str,
    fun: F,
    in_memory: Option<&Cache>,
) -> CustomResult<T, errors::StorageError>
where
    F: FnOnce() -> Fut + Send,
    Fut: futures::Future<Output = CustomResult<T, errors::StorageError>> + Send,
{
    let data = fun().await?;
    in_memory.async_map(|cache| cache.invalidate(key)).await;

    let redis_conn = store
        .get_redis_conn()
        .change_context(errors::StorageError::RedisError(
            RedisError::RedisConnectionError.into(),
        ))
        .attach_printable("Failed to get redis connection")?;

    redis_conn
        .delete_key(key)
        .await
        .change_context(errors::StorageError::KVError)?;
    Ok(data)
}

/// Asynchronously publishes the given cache keys into a redact channel using the provided storage interface.
///
/// # Arguments
///
/// * `store` - A reference to a storage interface that provides access to the necessary storage functionality.
/// * `keys` - An iterator of CacheKind instances that represent the cache keys to be published into the redact channel.
///
/// # Returns
///
/// A custom result containing the total number of cache keys successfully published into the redact channel, or a storage error if the operation fails.
pub async fn publish_into_redact_channel<'a, K: IntoIterator<Item = CacheKind<'a>> + Send>(
    store: &dyn StorageInterface,
    keys: K,
) -> CustomResult<usize, errors::StorageError> {
    let redis_conn = store
        .get_redis_conn()
        .change_context(errors::StorageError::RedisError(
            RedisError::RedisConnectionError.into(),
        ))
        .attach_printable("Failed to get redis connection")?;

    let futures = keys.into_iter().map(|key| async {
        redis_conn
            .clone()
            .publish(consts::PUB_SUB_CHANNEL, key)
            .await
            .change_context(errors::StorageError::KVError)
    });

    Ok(futures::future::try_join_all(futures)
        .await?
        .iter()
        .sum::<usize>())
}

/// Asynchronously publishes data into a storage and then redacts the published data. 
pub async fn publish_and_redact<'a, T, F, Fut>(
    store: &dyn StorageInterface,
    key: CacheKind<'a>,
    fun: F,
) -> CustomResult<T, errors::StorageError>
where
    F: FnOnce() -> Fut + Send,
    Fut: futures::Future<Output = CustomResult<T, errors::StorageError>> + Send,
{
    let data = fun().await?;
    publish_into_redact_channel(store, [key]).await?;
    Ok(data)
}

/// Asynchronously publishes data into a redact channel for multiple cache keys and returns the result.
pub async fn publish_and_redact_multiple<'a, T, F, Fut, K>(
    store: &dyn StorageInterface,
    keys: K,
    fun: F,
) -> CustomResult<T, errors::StorageError>
where
    F: FnOnce() -> Fut + Send,
    Fut: futures::Future<Output = CustomResult<T, errors::StorageError>> + Send,
    K: IntoIterator<Item = CacheKind<'a>> + Send,
{
    let data = fun().await?;
    publish_into_redact_channel(store, keys).await?;
    Ok(data)
}
