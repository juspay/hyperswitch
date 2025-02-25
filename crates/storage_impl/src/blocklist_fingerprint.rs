use common_utils::errors::CustomResult;
use diesel_models::blocklist_fingerprint as storage;
use error_stack::report;
use router_env::{instrument, tracing};
use sample::blocklist_fingerprint::BlocklistFingerprintInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> BlocklistFingerprintInterface for RouterStore<T> {
    type Error = errors::StorageError;
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
