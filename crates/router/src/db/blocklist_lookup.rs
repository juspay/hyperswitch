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
        /// Asynchronously inserts a new blocklist lookup entry into the storage.
    ///
    /// # Arguments
    ///
    /// * `blocklist_lookup_entry` - The new blocklist lookup entry to be inserted into the storage.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the inserted blocklist lookup entry or a `StorageError` if the insertion fails.
    ///
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

        /// Asynchronously finds a blocklist lookup entry by the provided merchant ID and fingerprint.
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - A string reference representing the merchant ID.
    /// * `fingerprint` - A string reference representing the fingerprint.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a `storage::BlocklistLookup` if successful, otherwise an `errors::StorageError`.
    ///
    async fn find_blocklist_lookup_entry_by_merchant_id_fingerprint(
        &self,
        merchant_id: &str,
        fingerprint: &str,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::BlocklistLookup::find_by_merchant_id_fingerprint(&conn, merchant_id, fingerprint)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously deletes a blocklist lookup entry from the storage by merchant ID and fingerprint.
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - A string reference representing the merchant ID.
    /// * `fingerprint` - A string reference representing the fingerprint.
    ///
    /// # Returns
    ///
    /// A custom result containing the deleted blocklist lookup entry or a storage error.
    ///
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
        /// Asynchronously inserts a new entry into the blocklist lookup table.
    ///
    /// # Arguments
    ///
    /// * `blocklist_lookup_entry` - The new blocklist lookup entry to be inserted into the database.
    ///
    /// # Returns
    ///
    /// * If successful, it returns the newly inserted blocklist lookup entry.
    /// * If an error occurs, it returns a `StorageError` indicating the cause of the error.
    ///
    async fn insert_blocklist_lookup_entry(
        &self,
        _blocklist_lookup_entry: storage::BlocklistLookupNew,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
        /// Asynchronously finds a blocklist lookup entry by merchant ID and fingerprint.
    ///
    /// # Arguments
    ///
    /// * `_merchant_id` - A string reference to the merchant ID
    /// * `_fingerprint` - A string reference to the fingerprint
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing either a `BlocklistLookup` or a `StorageError`
    ///
    async fn find_blocklist_lookup_entry_by_merchant_id_fingerprint(
        &self,
        _merchant_id: &str,
        _fingerprint: &str,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

        /// Asynchronously deletes a blocklist lookup entry by merchant ID and fingerprint.
    ///
    /// # Arguments
    /// * `_merchant_id` - A string reference representing the merchant ID.
    /// * `_fingerprint` - A string reference representing the fingerprint.
    ///
    /// # Returns
    /// A Result containing the deleted blocklist lookup entry or a StorageError if the operation fails.
    ///
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
        /// Asynchronously inserts a new entry into the blocklist lookup table.
    /// 
    /// # Arguments
    /// 
    /// * `blocklist_lookup_entry` - A `BlocklistLookupNew` struct containing the data for the new entry to be inserted.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing the newly inserted `BlocklistLookup` if successful, otherwise a `StorageError`.
    /// 
    async fn insert_blocklist_lookup_entry(
        &self,
        blocklist_lookup_entry: storage::BlocklistLookupNew,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
        self.diesel_store
            .insert_blocklist_lookup_entry(blocklist_lookup_entry)
            .await
    }

        /// Asynchronously finds a blocklist lookup entry by merchant ID and fingerprint in the database.
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - A string reference representing the ID of the merchant.
    /// * `fingerprint` - A string reference representing the fingerprint of the blocklist lookup entry.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a `storage::BlocklistLookup` if the entry is found, otherwise an `errors::StorageError`.
    ///
    async fn find_blocklist_lookup_entry_by_merchant_id_fingerprint(
        &self,
        merchant_id: &str,
        fingerprint: &str,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
        self.diesel_store
            .find_blocklist_lookup_entry_by_merchant_id_fingerprint(merchant_id, fingerprint)
            .await
    }

        /// Deletes a blocklist lookup entry by the provided merchant ID and fingerprint.
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - A string slice representing the merchant ID.
    /// * `fingerprint` - A string slice representing the fingerprint.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the result of the deletion operation, with the possible error being a `StorageError`.
    ///
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
