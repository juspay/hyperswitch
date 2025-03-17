use error_stack::ResultExt;
use redis_interface::errors::RedisError;
use scheduler::SchedulerInterface;
#[cfg(not(feature = "kv_store"))]
use storage_impl::RouterStore;
use storage_impl::{
    kv_router_store::KVRouterStore, redis::kv_store::RedisConnInterface, DatabaseStore, MockDb,
};

use crate::core::{
    domain::{customers::CustomerInterface, payment_methods::PaymentMethodInterface},
    errors::CustomResult,
};

#[async_trait::async_trait]
pub trait StorageInterface:
    Send
    + Sync
    + dyn_clone::DynClone
    + PaymentMethodInterface<Error = storage_impl::errors::StorageError>
    + CustomerInterface<Error = storage_impl::errors::StorageError>
    + SchedulerInterface
    + RedisConnInterface
    + 'static
{
    fn get_cache_store(&self) -> Box<(dyn RedisConnInterface + Send + Sync + 'static)>;
}
dyn_clone::clone_trait_object!(StorageInterface);

#[async_trait::async_trait]
impl StorageInterface for MockDb {
    fn get_cache_store(&self) -> Box<(dyn RedisConnInterface + Send + Sync + 'static)> {
        Box::new(self.clone())
    }
}

#[cfg(not(feature = "kv_store"))]
#[async_trait::async_trait]
impl<T: DatabaseStore + 'static> StorageInterface for RouterStore<T>
where
    RouterStore<T>:
        SchedulerInterface + CustomerInterface<Error = storage_impl::errors::StorageError>,
{
    fn get_cache_store(&self) -> Box<(dyn RedisConnInterface + Send + Sync + 'static)> {
        Box::new(self.clone())
    }
}

#[cfg(feature = "kv_store")]
#[async_trait::async_trait]
impl<T: DatabaseStore + 'static> StorageInterface for KVRouterStore<T>
where
    Self: SchedulerInterface + CustomerInterface<Error = storage_impl::errors::StorageError>,
{
    fn get_cache_store(&self) -> Box<(dyn RedisConnInterface + Send + Sync + 'static)> {
        Box::new(self.clone())
    }
}

pub async fn get_and_deserialize_key<T>(
    db: &dyn StorageInterface,
    key: &str,
    type_name: &'static str,
) -> CustomResult<T, RedisError>
where
    T: serde::de::DeserializeOwned,
{
    use common_utils::ext_traits::ByteSliceExt;

    let bytes = db.get_key(key).await?;
    bytes
        .parse_struct(type_name)
        .change_context(RedisError::JsonDeserializationFailed)
}
