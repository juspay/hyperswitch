use error_stack::report;
use router_env::instrument;
use storage_impl::MockDb;

use super::Store;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    db::kafka_store::KafkaStore,
    types::storage,
};

#[router_derive::track_async_interface]
#[async_trait::async_trait]
pub trait BlocklistFingerprintInterface {
    async fn insert_blocklist_fingerprint_entry(
        &self,
        pm_fingerprint_new: storage::BlocklistFingerprintNew,
    ) -> CustomResult<storage::BlocklistFingerprint, errors::StorageError>;

    async fn find_blocklist_fingerprint_by_merchant_id_fingerprint_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> CustomResult<storage::BlocklistFingerprint, errors::StorageError>;
}

#[async_trait::async_trait]
impl BlocklistFingerprintInterface for Store {
    #[instrument(skip_all)]
    async fn insert_blocklist_fingerprint_entry(
        &self,
        pm_fingerprint_new: storage::BlocklistFingerprintNew,
    ) -> CustomResult<storage::BlocklistFingerprint, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        pm_fingerprint_new
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_blocklist_fingerprint_by_merchant_id_fingerprint_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> CustomResult<storage::BlocklistFingerprint, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::BlocklistFingerprint::find_by_merchant_id_fingerprint_id(
            &conn,
            merchant_id,
            fingerprint_id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }
}

#[async_trait::async_trait]
impl BlocklistFingerprintInterface for MockDb {
    async fn insert_blocklist_fingerprint_entry(
        &self,
        _pm_fingerprint_new: storage::BlocklistFingerprintNew,
    ) -> CustomResult<storage::BlocklistFingerprint, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_blocklist_fingerprint_by_merchant_id_fingerprint_id(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        _fingerprint_id: &str,
    ) -> CustomResult<storage::BlocklistFingerprint, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}

#[async_trait::async_trait]
impl BlocklistFingerprintInterface for KafkaStore {
    #[instrument(skip_all)]
    async fn insert_blocklist_fingerprint_entry(
        &self,
        pm_fingerprint_new: storage::BlocklistFingerprintNew,
    ) -> CustomResult<storage::BlocklistFingerprint, errors::StorageError> {
        BlocklistFingerprintInterface::insert_blocklist_fingerprint_entry(
            &self.diesel_store,
            pm_fingerprint_new,
        )
        .await
    }

    #[instrument(skip_all)]
    async fn find_blocklist_fingerprint_by_merchant_id_fingerprint_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        fingerprint: &str,
    ) -> CustomResult<storage::BlocklistFingerprint, errors::StorageError> {
        BlocklistFingerprintInterface::find_blocklist_fingerprint_by_merchant_id_fingerprint_id(
            &self.diesel_store,
            merchant_id,
            fingerprint,
        )
        .await
    }
}
