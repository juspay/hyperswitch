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
pub trait PmBlocklistInterface {
    async fn insert_pm_blocklist_item(
        &self,
        pm_blocklist_new: storage::PmBlocklistNew,
    ) -> CustomResult<storage::PmBlocklist, errors::StorageError>;

    async fn find_pm_blocklist_entry_by_merchant_id_fingerprint(
        &self,
        merchant_id: String,
        hash: String,
    ) -> CustomResult<storage::PmBlocklist, errors::StorageError>;

    async fn delete_pm_blocklist_entry_by_merchant_id_hash(
        &self,
        merchant_id: String,
        hash: String,
    ) -> CustomResult<bool, errors::StorageError>;

    async fn list_all_blocked_pm_for_merchant(
        &self,
        merchant_id: String,
    ) -> CustomResult<Vec<storage::PmBlocklist>, errors::StorageError>;

    async fn list_all_blocked_pm_for_merchant_by_type(
        &self,
        merchant_id: String,
        pm_type: String,
    ) -> CustomResult<Vec<storage::PmBlocklist>, errors::StorageError>;
}

#[async_trait::async_trait]
impl PmBlocklistInterface for Store {
    #[instrument(skip_all)]
    async fn insert_pm_blocklist_item(
        &self,
        pm_blocklist: storage::PmBlocklistNew,
    ) -> CustomResult<storage::PmBlocklist, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        pm_blocklist
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_pm_blocklist_entry_by_merchant_id_fingerprint(
        &self,
        merchant_id: String,
        hash: String,
    ) -> CustomResult<storage::PmBlocklist, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::PmBlocklist::find_by_merchant_id_hash(&conn, merchant_id, hash)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn list_all_blocked_pm_for_merchant(
        &self,
        merchant_id: String,
    ) -> CustomResult<Vec<storage::PmBlocklist>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::PmBlocklist::find_by_merchant_id(&conn, merchant_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn list_all_blocked_pm_for_merchant_by_type(
        &self,
        merchant_id: String,
        pm_type: String,
    ) -> CustomResult<Vec<storage::PmBlocklist>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::PmBlocklist::find_by_merchant_id_type(&conn, merchant_id, pm_type)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn delete_pm_blocklist_entry_by_merchant_id_hash(
        &self,
        merchant_id: String,
        hash: String,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::PmBlocklist::delete_by_merchant_id_hash(&conn, merchant_id, hash)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

// TODO need to map this to either error or outputs
#[async_trait::async_trait]
impl PmBlocklistInterface for MockDb {
    #[instrument(skip_all)]
    async fn insert_pm_blocklist_item(
        &self,
        _pm_blocklist: storage::PmBlocklistNew,
    ) -> CustomResult<storage::PmBlocklist, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
    async fn find_pm_blocklist_entry_by_merchant_id_fingerprint(
        &self,
        _merchant_id: String,
        _hash: String,
    ) -> CustomResult<storage::PmBlocklist, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
    async fn list_all_blocked_pm_for_merchant(
        &self,
        _merchant_id: String,
    ) -> CustomResult<Vec<storage::PmBlocklist>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
    async fn list_all_blocked_pm_for_merchant_by_type(
        &self,
        _merchant_id: String,
        _pm_type: String,
    ) -> CustomResult<Vec<storage::PmBlocklist>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
    async fn delete_pm_blocklist_entry_by_merchant_id_hash(
        &self,
        _merchant_id: String,
        _hash: String,
    ) -> CustomResult<bool, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}

#[async_trait::async_trait]
impl PmBlocklistInterface for KafkaStore {
    #[instrument(skip_all)]
    async fn insert_pm_blocklist_item(
        &self,
        _pm_blocklist: storage::PmBlocklistNew,
    ) -> CustomResult<storage::PmBlocklist, errors::StorageError> {
        Err(errors::StorageError::KafkaError)?
    }
    async fn find_pm_blocklist_entry_by_merchant_id_fingerprint(
        &self,
        _merchant_id: String,
        _hash: String,
    ) -> CustomResult<storage::PmBlocklist, errors::StorageError> {
        Err(errors::StorageError::KafkaError)?
    }
    async fn delete_pm_blocklist_entry_by_merchant_id_hash(
        &self,
        _merchant_id: String,
        _hash: String,
    ) -> CustomResult<bool, errors::StorageError> {
        Err(errors::StorageError::KafkaError)?
    }
    async fn list_all_blocked_pm_for_merchant_by_type(
        &self,
        _merchant_id: String,
        _pm_type: String,
    ) -> CustomResult<Vec<storage::PmBlocklist>, errors::StorageError> {
        Err(errors::StorageError::KafkaError)?
    }
    async fn list_all_blocked_pm_for_merchant(
        &self,
        _merchant_id: String,
    ) -> CustomResult<Vec<storage::PmBlocklist>, errors::StorageError> {
        Err(errors::StorageError::KafkaError)?
    }
}

