use error_stack::report;
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
    //#\[instrument\(skip_all)]
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

    //#\[instrument\(skip_all)]
    async fn find_blocklist_entry_by_merchant_id_fingerprint_id(
        &self,
        merchant_id: &str,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Blocklist::find_by_merchant_id_fingerprint_id(&conn, merchant_id, fingerprint_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    //#\[instrument\(skip_all)]
    async fn list_blocklist_entries_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Blocklist::list_by_merchant_id(&conn, merchant_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    //#\[instrument\(skip_all)]
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
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    //#\[instrument\(skip_all)]
    async fn delete_blocklist_entry_by_merchant_id_fingerprint_id(
        &self,
        merchant_id: &str,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Blocklist::delete_by_merchant_id_fingerprint_id(&conn, merchant_id, fingerprint_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }
}

#[async_trait::async_trait]
impl BlocklistInterface for MockDb {
    //#\[instrument\(skip_all)]
    async fn insert_blocklist_entry(
        &self,
        _pm_blocklist: storage::BlocklistNew,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_blocklist_entry_by_merchant_id_fingerprint_id(
        &self,
        _merchant_id: &str,
        _fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn list_blocklist_entries_by_merchant_id(
        &self,
        _merchant_id: &str,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn list_blocklist_entries_by_merchant_id_data_kind(
        &self,
        _merchant_id: &str,
        _data_kind: common_enums::BlocklistDataKind,
        _limit: i64,
        _offset: i64,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

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
    //#\[instrument\(skip_all)]
    async fn insert_blocklist_entry(
        &self,
        pm_blocklist: storage::BlocklistNew,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        self.diesel_store.insert_blocklist_entry(pm_blocklist).await
    }

    //#\[instrument\(skip_all)]
    async fn find_blocklist_entry_by_merchant_id_fingerprint_id(
        &self,
        merchant_id: &str,
        fingerprint: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        self.diesel_store
            .find_blocklist_entry_by_merchant_id_fingerprint_id(merchant_id, fingerprint)
            .await
    }

    //#\[instrument\(skip_all)]
    async fn delete_blocklist_entry_by_merchant_id_fingerprint_id(
        &self,
        merchant_id: &str,
        fingerprint: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        self.diesel_store
            .delete_blocklist_entry_by_merchant_id_fingerprint_id(merchant_id, fingerprint)
            .await
    }

    //#\[instrument\(skip_all)]
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

    //#\[instrument\(skip_all)]
    async fn list_blocklist_entries_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        self.diesel_store
            .list_blocklist_entries_by_merchant_id(merchant_id)
            .await
    }
}
