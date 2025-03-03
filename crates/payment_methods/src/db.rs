use crate::core::domain::payment_methods::PaymentMethodInterface;
use crate::core::errors::CustomResult;
use error_stack::ResultExt;
use storage_impl::{kv_router_store::KVRouterStore, DatabaseStore, MockDb};
#[cfg(not(feature = "kv_store"))]
use storage_impl::RouterStore;
use redis_interface::errors::RedisError;

#[async_trait::async_trait]
pub trait StorageInterface:
    Send + Sync + dyn_clone::DynClone + PaymentMethodInterface + scheduler::SchedulerInterface + 'static
{
}
dyn_clone::clone_trait_object!(StorageInterface);

#[async_trait::async_trait]
impl StorageInterface for MockDb {}

#[cfg(not(feature = "kv_store"))]
#[async_trait::async_trait]
impl<T: DatabaseStore + 'static> StorageInterface for RouterStore<T> {}

#[cfg(feature = "kv_store")]
#[async_trait::async_trait]
impl<T: DatabaseStore + 'static> StorageInterface for KVRouterStore<T> where KVRouterStore<T>: scheduler::SchedulerInterface {}

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