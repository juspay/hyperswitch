use common_utils::errors::CustomResult;
use diesel_models::blocklist_lookup as storage;
use error_stack::report;
use router_env::{instrument, tracing};
use sample::blocklist_lookup::BlocklistLookupInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> BlocklistLookupInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn insert_blocklist_lookup_entry(
        &self,
        blocklist_lookup_entry: storage::BlocklistLookupNew,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        blocklist_lookup_entry
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_blocklist_lookup_entry_by_merchant_id_fingerprint(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        fingerprint: &str,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::BlocklistLookup::find_by_merchant_id_fingerprint(&conn, merchant_id, fingerprint)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn delete_blocklist_lookup_entry_by_merchant_id_fingerprint(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        fingerprint: &str,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::BlocklistLookup::delete_by_merchant_id_fingerprint(&conn, merchant_id, fingerprint)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }
}
