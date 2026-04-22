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

    async fn find_blocklist_entry_by_processor_merchant_id_fingerprint_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError>;

    async fn delete_blocklist_entry_by_processor_merchant_id_fingerprint_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError>;

    async fn list_blocklist_entries_by_processor_merchant_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError>;

    async fn list_blocklist_entries_by_processor_merchant_id_data_kind(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        data_kind: common_enums::BlocklistDataKind,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError>;

    async fn get_blocklist_entries_count_by_processor_merchant_id_data_kind(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        data_kind: common_enums::BlocklistDataKind,
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
    async fn find_blocklist_entry_by_processor_merchant_id_fingerprint_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        // Stagger release fallback: first try processor_merchant_id, if not found fallback to merchant_id
        // For old records processor_merchant_id is NULL, so we use merchant_id (which has the same value)
        let result = storage::Blocklist::find_by_processor_merchant_id_fingerprint_id(
            &conn,
            processor_merchant_id,
            fingerprint_id,
        )
        .await;

        match result {
            Ok(blocklist_entry) => Ok(blocklist_entry),
            Err(error) => {
                if matches!(
                    error.current_context(),
                    diesel_models::errors::DatabaseError::NotFound
                ) {
                    storage::Blocklist::find_by_merchant_id_fingerprint_id(
                        &conn,
                        processor_merchant_id,
                        fingerprint_id,
                    )
                    .await
                    .map_err(|error| report!(errors::StorageError::from(error)))
                } else {
                    Err(report!(errors::StorageError::from(error)))
                }
            }
        }
    }

    #[instrument(skip_all)]
    async fn list_blocklist_entries_by_processor_merchant_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Blocklist::list_by_processor_merchant_id(&conn, processor_merchant_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_blocklist_entries_by_processor_merchant_id_data_kind(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        data_kind: common_enums::BlocklistDataKind,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Blocklist::list_by_processor_merchant_id_data_kind(
            &conn,
            processor_merchant_id,
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
    async fn get_blocklist_entries_count_by_processor_merchant_id_data_kind(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        data_kind: common_enums::BlocklistDataKind,
    ) -> CustomResult<usize, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Blocklist::get_count_by_processor_merchant_id_data_kind(
            &conn,
            processor_merchant_id,
            data_kind,
        )
        .await
        .change_context(errors::StorageError::DatabaseError(report!(
            diesel_models::errors::DatabaseError::Others
        )))
    }

    #[instrument(skip_all)]
    async fn delete_blocklist_entry_by_processor_merchant_id_fingerprint_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        // Stagger release fallback: first try processor_merchant_id, if not found fallback to merchant_id
        // For old records processor_merchant_id is NULL, so we delete by merchant_id (which has the same value)
        let result = storage::Blocklist::delete_by_processor_merchant_id_fingerprint_id(
            &conn,
            processor_merchant_id,
            fingerprint_id,
        )
        .await;

        match result {
            Ok(blocklist) => Ok(blocklist),
            Err(error) => {
                if matches!(
                    error.current_context(),
                    diesel_models::errors::DatabaseError::NotFound
                ) {
                    storage::Blocklist::delete_by_merchant_id_fingerprint_id(
                        &conn,
                        processor_merchant_id,
                        fingerprint_id,
                    )
                    .await
                    .map_err(|error| report!(errors::StorageError::from(error)))
                } else {
                    Err(report!(errors::StorageError::from(error)))
                }
            }
        }
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

    async fn find_blocklist_entry_by_processor_merchant_id_fingerprint_id(
        &self,
        _processor_merchant_id: &common_utils::id_type::MerchantId,
        _fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn list_blocklist_entries_by_processor_merchant_id(
        &self,
        _processor_merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn list_blocklist_entries_by_processor_merchant_id_data_kind(
        &self,
        _processor_merchant_id: &common_utils::id_type::MerchantId,
        _data_kind: common_enums::BlocklistDataKind,
        _limit: i64,
        _offset: i64,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn get_blocklist_entries_count_by_processor_merchant_id_data_kind(
        &self,
        _processor_merchant_id: &common_utils::id_type::MerchantId,
        _data_kind: common_enums::BlocklistDataKind,
    ) -> CustomResult<usize, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn delete_blocklist_entry_by_processor_merchant_id_fingerprint_id(
        &self,
        _processor_merchant_id: &common_utils::id_type::MerchantId,
        _fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
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
    async fn find_blocklist_entry_by_processor_merchant_id_fingerprint_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        self.diesel_store
            .find_blocklist_entry_by_processor_merchant_id_fingerprint_id(processor_merchant_id, fingerprint_id)
            .await
    }

    #[instrument(skip_all)]
    async fn delete_blocklist_entry_by_processor_merchant_id_fingerprint_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        self.diesel_store
            .delete_blocklist_entry_by_processor_merchant_id_fingerprint_id(processor_merchant_id, fingerprint_id)
            .await
    }

    #[instrument(skip_all)]
    async fn list_blocklist_entries_by_processor_merchant_id_data_kind(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        data_kind: common_enums::BlocklistDataKind,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        self.diesel_store
            .list_blocklist_entries_by_processor_merchant_id_data_kind(processor_merchant_id, data_kind, limit, offset)
            .await
    }

    #[instrument(skip_all)]
    async fn get_blocklist_entries_count_by_processor_merchant_id_data_kind(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        data_kind: common_enums::BlocklistDataKind,
    ) -> CustomResult<usize, errors::StorageError> {
        self.diesel_store
            .get_blocklist_entries_count_by_processor_merchant_id_data_kind(processor_merchant_id, data_kind)
            .await
    }

    #[instrument(skip_all)]
    async fn list_blocklist_entries_by_processor_merchant_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        self.diesel_store
            .list_blocklist_entries_by_processor_merchant_id(processor_merchant_id)
            .await
    }
}
