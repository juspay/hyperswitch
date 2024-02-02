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
pub trait BlocklistInterface {
    async fn insert_blocklist_entry(
        &self,
        pm_blocklist_new: storage::BlocklistNew,
    ) -> CustomResult<storage::Blocklist, errors::StorageError>;

    async fn find_blocklist_entry_by_merchant_id_fingerprint_id(
        &self,
        merchant_id: &str,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError>;

    async fn delete_blocklist_entry_by_merchant_id_fingerprint_id(
        &self,
        merchant_id: &str,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError>;

    async fn list_blocklist_entries_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError>;

    async fn list_blocklist_entries_by_merchant_id_data_kind(
        &self,
        merchant_id: &str,
        data_kind: common_enums::BlocklistDataKind,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError>;
}

#[async_trait::async_trait]
impl BlocklistInterface for Store {
    #[instrument(skip_all)]
        /// Asynchronously inserts a new entry into the blocklist table in the database.
    ///
    /// # Arguments
    /// * `pm_blocklist` - The new blocklist entry to be inserted into the database.
    ///
    /// # Returns
    /// A `CustomResult` containing the newly inserted blocklist entry if successful, or a `StorageError` if an error occurs during the insertion process.
    ///
    async fn insert_blocklist_entry(
        &self,
        pm_blocklist: storage::BlocklistNew,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        pm_blocklist
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously finds a blocklist entry by the given merchant ID and fingerprint ID.
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - A string slice representing the merchant ID to search for.
    /// * `fingerprint_id` - A string slice representing the fingerprint ID to search for.
    ///
    /// # Returns
    ///
    /// A Result containing a storage::Blocklist if the entry is found, or a errors::StorageError if an error occurs.
    ///
    async fn find_blocklist_entry_by_merchant_id_fingerprint_id(
        &self,
        merchant_id: &str,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Blocklist::find_by_merchant_id_fingerprint_id(&conn, merchant_id, fingerprint_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Retrieves a list of blocklist entries for a given merchant ID from the storage.
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - A reference to a string containing the merchant ID.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a vector of `storage::Blocklist` if successful, otherwise an `errors::StorageError`.
    ///
    async fn list_blocklist_entries_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Blocklist::list_by_merchant_id(&conn, merchant_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Retrieves a list of blocklist entries by merchant ID and data kind with a specified limit and offset.
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - The ID of the merchant for which to retrieve blocklist entries.
    /// * `data_kind` - The kind of data for which to retrieve blocklist entries.
    /// * `limit` - The maximum number of blocklist entries to retrieve.
    /// * `offset` - The number of blocklist entries to skip before starting to return items.
    ///
    /// # Returns
    ///
    /// A vector of `storage::Blocklist` containing the retrieved blocklist entries, wrapped in a `CustomResult` representing success or an error of type `errors::StorageError`.
    ///
    async fn list_blocklist_entries_by_merchant_id_data_kind(
        &self,
        merchant_id: &str,
        data_kind: common_enums::BlocklistDataKind,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Blocklist::list_by_merchant_id_data_kind(
            &conn,
            merchant_id,
            data_kind,
            limit,
            offset,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

        /// Asynchronously deletes a blocklist entry by merchant ID and fingerprint ID.
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - A string slice representing the merchant ID.
    /// * `fingerprint_id` - A string slice representing the fingerprint ID.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the result of the delete operation, or a `StorageError` if an error occurs.
    ///
    async fn delete_blocklist_entry_by_merchant_id_fingerprint_id(
        &self,
        merchant_id: &str,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Blocklist::delete_by_merchant_id_fingerprint_id(&conn, merchant_id, fingerprint_id)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl BlocklistInterface for MockDb {
    #[instrument(skip_all)]
        /// Asynchronously inserts a new entry into the blocklist and returns the inserted blocklist entry. 
    ///
    /// # Arguments
    ///
    /// * `_pm_blocklist` - The new blocklist entry to be inserted into the database.
    ///
    /// # Returns
    ///
    /// * `CustomResult<storage::Blocklist, errors::StorageError>` - A result indicating success with the inserted blocklist entry or an error if the insertion fails.
    ///
    async fn insert_blocklist_entry(
        &self,
        _pm_blocklist: storage::BlocklistNew,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

        /// Asynchronously finds a blocklist entry by merchant ID and fingerprint ID.
    ///
    /// # Arguments
    ///
    /// * `_merchant_id` - A reference to a string representing the merchant ID.
    /// * `_fingerprint_id` - A reference to a string representing the fingerprint ID.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the found `storage::Blocklist` if successful, otherwise a `StorageError`.
    ///
    async fn find_blocklist_entry_by_merchant_id_fingerprint_id(
        &self,
        _merchant_id: &str,
        _fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

        /// Asynchronously retrieves a list of blocklist entries associated with a specific merchant ID from the storage.
    ///
    /// # Arguments
    ///
    /// * `_merchant_id` - A reference to a string representing the merchant ID.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a vector of `storage::Blocklist` if successful, otherwise an `errors::StorageError`.
    ///
    /// # Errors
    ///
    /// If an error occurs during the retrieval process, an `errors::StorageError::MockDbError` is returned.
    ///
    async fn list_blocklist_entries_by_merchant_id(
        &self,
        _merchant_id: &str,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

        /// Retrieves a list of blocklist entries for a specific merchant ID and data kind,
    /// with a specified limit and offset. Returns a vector of `Blocklist` items
    /// wrapped in a `CustomResult`. If an error occurs during database operation,
    /// a `StorageError` is returned.
    async fn list_blocklist_entries_by_merchant_id_data_kind(
        &self,
        _merchant_id: &str,
        _data_kind: common_enums::BlocklistDataKind,
        _limit: i64,
        _offset: i64,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

        /// Asynchronously deletes a blocklist entry by merchant ID and fingerprint ID. 
    ///
    /// # Arguments
    /// 
    /// * `_merchant_id` - A string reference representing the merchant ID of the blocklist entry to be deleted.
    /// * `_fingerprint_id` - A string reference representing the fingerprint ID of the blocklist entry to be deleted.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the result of the operation. If successful, it returns the deleted blocklist entry. If unsuccessful, it returns a `StorageError`.
    ///
    async fn delete_blocklist_entry_by_merchant_id_fingerprint_id(
        &self,
        _merchant_id: &str,
        _fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}

#[async_trait::async_trait]
impl BlocklistInterface for KafkaStore {
    #[instrument(skip_all)]
        /// Asynchronously inserts a new entry into the blocklist using the provided data.
    ///
    /// # Arguments
    ///
    /// * `pm_blocklist` - The blocklist entry to be inserted.
    ///
    /// # Returns
    ///
    /// * `CustomResult<storage::Blocklist, errors::StorageError>` - A result indicating success or failure, containing the inserted blocklist entry or an error.
    ///
    async fn insert_blocklist_entry(
        &self,
        pm_blocklist: storage::BlocklistNew,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        self.diesel_store.insert_blocklist_entry(pm_blocklist).await
    }

        /// Asynchronously finds a blocklist entry in the storage by merchant ID and fingerprint ID.
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - A reference to a string representing the merchant ID.
    /// * `fingerprint` - A reference to a string representing the fingerprint ID.
    ///
    /// # Returns
    ///
    /// A `CustomResult` with the result of the operation, containing a `Blocklist` if successful, or a `StorageError` if an error occurred.
    ///
    async fn find_blocklist_entry_by_merchant_id_fingerprint_id(
        &self,
        merchant_id: &str,
        fingerprint: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        self.diesel_store
            .find_blocklist_entry_by_merchant_id_fingerprint_id(merchant_id, fingerprint)
            .await
    }

        /// Asynchronously deletes a blocklist entry from the storage by merchant ID and fingerprint ID.
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - A reference to a string representing the merchant ID.
    /// * `fingerprint` - A reference to a string representing the fingerprint ID.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a `storage::Blocklist` if successful, otherwise an `errors::StorageError`.
    ///
    async fn delete_blocklist_entry_by_merchant_id_fingerprint_id(
        &self,
        merchant_id: &str,
        fingerprint: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        self.diesel_store
            .delete_blocklist_entry_by_merchant_id_fingerprint_id(merchant_id, fingerprint)
            .await
    }

        /// Retrieves a list of blocklist entries for a specific merchant and data kind, with the option to limit the results and apply an offset.
        async fn list_blocklist_entries_by_merchant_id_data_kind(
            &self,
            merchant_id: &str,
            data_kind: common_enums::BlocklistDataKind,
            limit: i64,
            offset: i64,
        ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
            self.diesel_store
                .list_blocklist_entries_by_merchant_id_data_kind(merchant_id, data_kind, limit, offset)
                .await
        }
    /// Asynchronously retrieves a list of blocklist entries for a specific merchant by their ID.
    /// 
    /// # Arguments
    /// 
    /// * `merchant_id` - A string reference representing the ID of the merchant
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing a vector of `storage::Blocklist` if successful, otherwise an `errors::StorageError`
    /// 
    async fn list_blocklist_entries_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        self.diesel_store
            .list_blocklist_entries_by_merchant_id(merchant_id)
            .await
    }
}
