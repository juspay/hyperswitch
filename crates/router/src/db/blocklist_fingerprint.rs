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
pub trait BlocklistFingerprintInterface {
    async fn insert_blocklist_fingerprint_entry(
        &self,
        pm_fingerprint_new: storage::BlocklistFingerprintNew,
    ) -> CustomResult<storage::BlocklistFingerprint, errors::StorageError>;

    async fn find_blocklist_fingerprint_by_merchant_id_fingerprint_id(
        &self,
        merchant_id: &str,
        fingerprint_id: &str,
    ) -> CustomResult<storage::BlocklistFingerprint, errors::StorageError>;
}

#[async_trait::async_trait]
impl BlocklistFingerprintInterface for Store {
    #[instrument(skip_all)]
        /// Inserts a new blocklist fingerprint entry into the database.
    /// 
    /// # Arguments
    /// 
    /// * `pm_fingerprint_new` - A `BlocklistFingerprintNew` object containing the data for the new entry.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing the newly inserted `BlocklistFingerprint` or a `StorageError` if the insertion fails.
    /// 
    async fn insert_blocklist_fingerprint_entry(
        &self,
        pm_fingerprint_new: storage::BlocklistFingerprintNew,
    ) -> CustomResult<storage::BlocklistFingerprint, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        pm_fingerprint_new
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously finds a blocklist fingerprint by merchant ID and fingerprint ID.
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - A string reference representing the merchant ID.
    /// * `fingerprint_id` - A string reference representing the fingerprint ID.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the `BlocklistFingerprint` if found, otherwise a `StorageError`.
    ///
    async fn find_blocklist_fingerprint_by_merchant_id_fingerprint_id(
        &self,
        merchant_id: &str,
        fingerprint_id: &str,
    ) -> CustomResult<storage::BlocklistFingerprint, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::BlocklistFingerprint::find_by_merchant_id_fingerprint_id(
            &conn,
            merchant_id,
            fingerprint_id,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }
}

#[async_trait::async_trait]
impl BlocklistFingerprintInterface for MockDb {
    #[instrument(skip_all)]
        /// Inserts a new blocklist fingerprint entry into the storage.
    ///
    /// # Arguments
    ///
    /// * `_pm_fingerprint_new` - The new blocklist fingerprint entry to be inserted.
    ///
    /// # Returns
    ///
    /// * `CustomResult<storage::BlocklistFingerprint, errors::StorageError>` - A result indicating success with the inserted blocklist fingerprint or an error if the insertion fails.
    ///
    /// # Errors
    ///
    /// This method always returns an error of type `errors::StorageError::MockDbError`.
    ///
    async fn insert_blocklist_fingerprint_entry(
        &self,
        _pm_fingerprint_new: storage::BlocklistFingerprintNew,
    ) -> CustomResult<storage::BlocklistFingerprint, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

        /// Asynchronously finds a blocklist fingerprint by the given merchant ID and fingerprint ID.
    /// If found, returns a `CustomResult` containing the `BlocklistFingerprint`, otherwise returns a `StorageError`.
    async fn find_blocklist_fingerprint_by_merchant_id_fingerprint_id(
        &self,
        _merchant_id: &str,
        _fingerprint_id: &str,
    ) -> CustomResult<storage::BlocklistFingerprint, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}

#[async_trait::async_trait]
impl BlocklistFingerprintInterface for KafkaStore {
    #[instrument(skip_all)]
        /// Asynchronously inserts a new blocklist fingerprint entry into the storage using the provided BlocklistFingerprintNew object.
    ///
    /// # Arguments
    ///
    /// * `pm_fingerprint_new` - The new blocklist fingerprint entry to be inserted into the storage.
    ///
    /// # Returns
    ///
    /// A CustomResult containing the inserted blocklist fingerprint entry on success, or a StorageError on failure.
    ///
    async fn insert_blocklist_fingerprint_entry(
        &self,
        pm_fingerprint_new: storage::BlocklistFingerprintNew,
    ) -> CustomResult<storage::BlocklistFingerprint, errors::StorageError> {
        self.diesel_store
            .insert_blocklist_fingerprint_entry(pm_fingerprint_new)
            .await
    }

        /// Asynchronously finds a blocklist fingerprint by merchant ID and fingerprint ID in the storage.
    /// Returns a `CustomResult` containing a `BlocklistFingerprint` if successful, or a `StorageError` if an error occurs.
    async fn find_blocklist_fingerprint_by_merchant_id_fingerprint_id(
        &self,
        merchant_id: &str,
        fingerprint: &str,
    ) -> CustomResult<storage::BlocklistFingerprint, errors::StorageError> {
        self.diesel_store
            .find_blocklist_fingerprint_by_merchant_id_fingerprint_id(merchant_id, fingerprint)
            .await
    }
}
