use error_stack::{report, ResultExt};
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
        merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError>;

    async fn delete_blocklist_entry_by_merchant_id_fingerprint_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError>;

    async fn list_blocklist_entries_by_merchant_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError>;

    async fn list_blocklist_entries_by_merchant_id_data_kind(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        data_kind: common_enums::BlocklistDataKind,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError>;

    async fn get_blocklist_entries_count_by_merchant_id_data_kind(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        data_kind: common_enums::BlocklistDataKind,
    ) -> CustomResult<usize, errors::StorageError>;

    /// Bulk insert entries with `ON CONFLICT DO NOTHING`.
    /// Returns the number of rows *newly* inserted.
    /// Rows already present are silently skipped (not an error).
    async fn bulk_insert_blocklist_entries(
        &self,
        entries: Vec<storage::BlocklistNew>,
    ) -> CustomResult<usize, errors::StorageError>;
}

#[async_trait::async_trait]
impl BlocklistInterface for Store {
    #[instrument(skip_all)]
    async fn insert_blocklist_entry(
        &self,
        pm_blocklist: storage::BlocklistNew,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        pm_blocklist
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_blocklist_entry_by_merchant_id_fingerprint_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Blocklist::find_by_merchant_id_fingerprint_id(&conn, merchant_id, fingerprint_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_blocklist_entries_by_merchant_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Blocklist::list_by_merchant_id(&conn, merchant_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_blocklist_entries_by_merchant_id_data_kind(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        data_kind: common_enums::BlocklistDataKind,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Blocklist::list_by_merchant_id_data_kind(
            &conn,
            merchant_id,
            data_kind,
            limit,
            offset,
        )
        .await
        .change_context(errors::StorageError::DatabaseError(report!(
            diesel_models::errors::DatabaseError::Others
        )))
    }

    #[instrument(skip_all)]
    async fn get_blocklist_entries_count_by_merchant_id_data_kind(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        data_kind: common_enums::BlocklistDataKind,
    ) -> CustomResult<usize, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Blocklist::get_count_by_merchant_id_data_kind(&conn, merchant_id, data_kind)
            .await
            .change_context(errors::StorageError::DatabaseError(report!(
                diesel_models::errors::DatabaseError::Others
            )))
    }

    #[instrument(skip_all)]
    async fn delete_blocklist_entry_by_merchant_id_fingerprint_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Blocklist::delete_by_merchant_id_fingerprint_id(&conn, merchant_id, fingerprint_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn bulk_insert_blocklist_entries(
        &self,
        entries: Vec<storage::BlocklistNew>,
    ) -> CustomResult<usize, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::BlocklistNew::bulk_insert_on_conflict_do_nothing(&conn, entries)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }
}

#[async_trait::async_trait]
impl BlocklistInterface for MockDb {
    #[instrument(skip_all)]
    async fn insert_blocklist_entry(
        &self,
        _pm_blocklist: storage::BlocklistNew,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_blocklist_entry_by_merchant_id_fingerprint_id(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        _fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn list_blocklist_entries_by_merchant_id(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn list_blocklist_entries_by_merchant_id_data_kind(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        _data_kind: common_enums::BlocklistDataKind,
        _limit: i64,
        _offset: i64,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn get_blocklist_entries_count_by_merchant_id_data_kind(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        _data_kind: common_enums::BlocklistDataKind,
    ) -> CustomResult<usize, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn delete_blocklist_entry_by_merchant_id_fingerprint_id(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        _fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn bulk_insert_blocklist_entries(
        &self,
        entries: Vec<storage::BlocklistNew>,
    ) -> CustomResult<usize, errors::StorageError> {
        let mut blocklists = self.blocklists.lock().await;
        let mut inserted = 0usize;
        for entry in entries {
            let already_exists = blocklists.iter().any(|b| {
                b.merchant_id == entry.merchant_id && b.fingerprint_id == entry.fingerprint_id
            });
            if !already_exists {
                blocklists.push(storage::Blocklist {
                    merchant_id: entry.merchant_id,
                    fingerprint_id: entry.fingerprint_id,
                    data_kind: entry.data_kind,
                    metadata: entry.metadata,
                    created_at: entry.created_at,
                });
                inserted += 1;
            }
        }
        Ok(inserted)
    }
}

#[async_trait::async_trait]
impl BlocklistInterface for KafkaStore {
    #[instrument(skip_all)]
    async fn insert_blocklist_entry(
        &self,
        pm_blocklist: storage::BlocklistNew,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        self.diesel_store.insert_blocklist_entry(pm_blocklist).await
    }

    #[instrument(skip_all)]
    async fn find_blocklist_entry_by_merchant_id_fingerprint_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        fingerprint: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        self.diesel_store
            .find_blocklist_entry_by_merchant_id_fingerprint_id(merchant_id, fingerprint)
            .await
    }

    #[instrument(skip_all)]
    async fn delete_blocklist_entry_by_merchant_id_fingerprint_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        fingerprint: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        self.diesel_store
            .delete_blocklist_entry_by_merchant_id_fingerprint_id(merchant_id, fingerprint)
            .await
    }

    #[instrument(skip_all)]
    async fn list_blocklist_entries_by_merchant_id_data_kind(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        data_kind: common_enums::BlocklistDataKind,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        self.diesel_store
            .list_blocklist_entries_by_merchant_id_data_kind(merchant_id, data_kind, limit, offset)
            .await
    }

    #[instrument(skip_all)]
    async fn get_blocklist_entries_count_by_merchant_id_data_kind(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        data_kind: common_enums::BlocklistDataKind,
    ) -> CustomResult<usize, errors::StorageError> {
        self.diesel_store
            .get_blocklist_entries_count_by_merchant_id_data_kind(merchant_id, data_kind)
            .await
    }

    #[instrument(skip_all)]
    async fn list_blocklist_entries_by_merchant_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        self.diesel_store
            .list_blocklist_entries_by_merchant_id(merchant_id)
            .await
    }

    #[instrument(skip_all)]
    async fn bulk_insert_blocklist_entries(
        &self,
        entries: Vec<storage::BlocklistNew>,
    ) -> CustomResult<usize, errors::StorageError> {
        self.diesel_store
            .bulk_insert_blocklist_entries(entries)
            .await
    }
}
