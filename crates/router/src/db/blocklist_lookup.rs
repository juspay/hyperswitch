use error_stack::IntoReport;
use router_env::{instrument, tracing};
use storage_impl::MockDb;

use super::Store;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    db::kafka_store::KafkaStore,
    types::storage,
};

#[async_trait::async_trait]
pub trait BlocklistLookupInterface {
    async fn insert_blocklist_lookup_entry(
        &self,
        blocklist_lookup_new: storage::BlocklistLookupNew,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError>;

    async fn find_blocklist_lookup_entry_by_merchant_id_fingerprint(
        &self,
        merchant_id: &str,
        fingerprint: &str,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError>;

    async fn delete_blocklist_lookup_entry_by_merchant_id_fingerprint(
        &self,
        merchant_id: &str,
        fingerprint: &str,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError>;
}

#[async_trait::async_trait]
impl BlocklistLookupInterface for Store {
    #[instrument(skip_all)]
    async fn insert_blocklist_lookup_entry(
        &self,
        blocklist_lookup_entry: storage::BlocklistLookupNew,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        blocklist_lookup_entry
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

    #[instrument(skip_all)]
    async fn find_blocklist_lookup_entry_by_merchant_id_fingerprint(
        &self,
        merchant_id: &str,
        fingerprint: &str,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::BlocklistLookup::find_by_merchant_id_fingerprint(&conn, merchant_id, fingerprint)
            .await
            .map_err(Into::into)
            .into_report()
    }

    #[instrument(skip_all)]
    async fn delete_blocklist_lookup_entry_by_merchant_id_fingerprint(
        &self,
        merchant_id: &str,
        fingerprint: &str,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::BlocklistLookup::delete_by_merchant_id_fingerprint(&conn, merchant_id, fingerprint)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl BlocklistLookupInterface for MockDb {
    #[instrument(skip_all)]
    async fn insert_blocklist_lookup_entry(
        &self,
        _blocklist_lookup_entry: storage::BlocklistLookupNew,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_blocklist_lookup_entry_by_merchant_id_fingerprint(
        &self,
        _merchant_id: &str,
        _fingerprint: &str,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn delete_blocklist_lookup_entry_by_merchant_id_fingerprint(
        &self,
        _merchant_id: &str,
        _fingerprint: &str,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}

#[async_trait::async_trait]
impl BlocklistLookupInterface for KafkaStore {
    #[instrument(skip_all)]
    async fn insert_blocklist_lookup_entry(
        &self,
        blocklist_lookup_entry: storage::BlocklistLookupNew,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
        self.diesel_store
            .insert_blocklist_lookup_entry(blocklist_lookup_entry)
            .await
    }

    #[instrument(skip_all)]
    async fn find_blocklist_lookup_entry_by_merchant_id_fingerprint(
        &self,
        merchant_id: &str,
        fingerprint: &str,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
        self.diesel_store
            .find_blocklist_lookup_entry_by_merchant_id_fingerprint(merchant_id, fingerprint)
            .await
    }

    #[instrument(skip_all)]
    async fn delete_blocklist_lookup_entry_by_merchant_id_fingerprint(
        &self,
        merchant_id: &str,
        fingerprint: &str,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
        self.diesel_store
            .delete_blocklist_lookup_entry_by_merchant_id_fingerprint(merchant_id, fingerprint)
            .await
    }
}
