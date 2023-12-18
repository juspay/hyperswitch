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
pub trait PmFingerprintInterface {
    async fn insert_pm_fingerprint_entry(
        &self,
        pm_fingerprint_new: storage::PmFingerprintNew,
    ) -> CustomResult<storage::PmFingerprint, errors::StorageError>;

    async fn find_pm_fingerprint_entry(
        &self,
        fingerprint: String,
    ) -> CustomResult<storage::PmFingerprint, errors::StorageError>;

    async fn delete_pm_fingerprint_entry(
        &self,
        fingerprint: String,
    ) -> CustomResult<bool, errors::StorageError>;
}

#[async_trait::async_trait]
impl PmFingerprintInterface for Store {
    #[instrument(skip_all)]
    async fn insert_pm_fingerprint_entry(
        &self,
        pm_fingerprint_new: storage::PmFingerprintNew,
    ) -> CustomResult<storage::PmFingerprint, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        pm_fingerprint_new
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_pm_fingerprint_entry(
        &self,
        fingerprint: String,
    ) -> CustomResult<storage::PmFingerprint, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::PmFingerprint::find_by_fingerprint(&conn, fingerprint)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn delete_pm_fingerprint_entry(
        &self,
        fingerprint: String,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::PmFingerprint::delete_by_fingerprint(&conn, fingerprint)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

// // TODO need to map this to either error or outputs
#[async_trait::async_trait]
impl PmFingerprintInterface for MockDb {
    #[instrument(skip_all)]
    async fn insert_pm_fingerprint_entry(
        &self,
        pm_fingerprint_new: storage::PmFingerprintNew,
    ) -> CustomResult<storage::PmFingerprint, errors::StorageError> {
        Ok(storage::PmFingerprint {
            id: 4,
            fingerprint_id: "1234".to_string(),
            kms_hash: "hash".to_string(),
        })
    }

    async fn find_pm_fingerprint_entry(
        &self,
        fingerprint: String,
    ) -> CustomResult<storage::PmFingerprint, errors::StorageError> {
        Ok(storage::PmFingerprint {
            id: 4,
            fingerprint_id: "1234".to_string(),
            kms_hash: "hash".to_string(),
        })
    }
    async fn delete_pm_fingerprint_entry(
        &self,
        fingerprint: String,
    ) -> CustomResult<bool, errors::StorageError> {
        Ok(true)
    }
}

#[async_trait::async_trait]
impl PmFingerprintInterface for KafkaStore {
    #[instrument(skip_all)]
    async fn insert_pm_fingerprint_entry(
        &self,
        pm_fingerprint_new: storage::PmFingerprintNew,
    ) -> CustomResult<storage::PmFingerprint, errors::StorageError> {
        Ok(storage::PmFingerprint {
            id: 4,
            fingerprint_id: "1234".to_string(),
            kms_hash: "hash".to_string(),
        })
    }

    async fn find_pm_fingerprint_entry(
        &self,
        fingerprint: String,
    ) -> CustomResult<storage::PmFingerprint, errors::StorageError> {
        Ok(storage::PmFingerprint {
            id: 4,
            fingerprint_id: "1234".to_string(),
            kms_hash: "hash".to_string(),
        })
    }
    async fn delete_pm_fingerprint_entry(
        &self,
        fingerprint: String,
    ) -> CustomResult<bool, errors::StorageError> {
        Ok(true)
    }
}
