use crate::redis::kv_store::KvStorePartition;

impl KvStorePartition for storage::ReverseLookup {}

// #[cfg(not(feature = "kv_store"))]
// mod storage {
use common_utils::errors::CustomResult;
use diesel_models::{enums, reverse_lookup as storage};
use error_stack::report;
use router_env::{instrument, tracing};
use sample::reverse_lookup::ReverseLookupInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> ReverseLookupInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn insert_reverse_lookup(
        &self,
        new: storage::ReverseLookupNew,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage::ReverseLookup, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        new.insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn get_lookup_by_lookup_id(
        &self,
        id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage::ReverseLookup, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::ReverseLookup::find_by_lookup_id(id, &conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }
}
// }
