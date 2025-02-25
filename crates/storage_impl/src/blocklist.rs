use common_utils::errors::CustomResult;
use diesel_models::blocklist as storage;
use error_stack::report;
use router_env::{instrument, tracing};
use sample::blocklist::BlocklistInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> BlocklistInterface for RouterStore<T> {
    type Error = errors::StorageError;

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
}
