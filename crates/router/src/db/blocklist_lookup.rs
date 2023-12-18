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
pub trait BlocklistLookupInterface {
    async fn insert_blocklist_lookup_entry(
        &self,
        blocklist_lookup_new: storage::BlocklistLookupNew,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError>;

    async fn find_blocklist_lookup_entry_by_merchant_id_kms_decrypted_hash(
        &self,
        merchant_id: String,
        kms_decrypted_hash: String,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError>;

    async fn delete_blocklist_lookup_entry_by_merchant_id_kms_decrypted_hash(
        &self,
        merchant_id: String,
        kms_decrypted_hash: String,
    ) -> CustomResult<bool, errors::StorageError>;
}

#[async_trait::async_trait]
impl BlocklistLookupInterface for Store {
    #[instrument(skip_all)]
    async fn insert_blocklist_lookup_entry(
        &self,
        blocklist_lookup_entry: storage::BlocklistLookupNew,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        blocklist_lookup_entry
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_blocklist_lookup_entry_by_merchant_id_kms_decrypted_hash (
        &self,
        merchant_id: String,
        kms_decrypted_hash: String,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::BlocklistLookup::find_by_merchant_id_kms_encrypted_hash(&conn, merchant_id, kms_decrypted_hash)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn delete_blocklist_lookup_entry_by_merchant_id_kms_decrypted_hash (
        &self,
        merchant_id: String,
        kms_decrypted_hash: String,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::BlocklistLookup::delete_by_merchant_id_kms_decrypted_hash(&conn, merchant_id, kms_decrypted_hash)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

// TODO need to map this to either error or outputs
#[async_trait::async_trait]
impl BlocklistLookupInterface for MockDb {
    #[instrument(skip_all)]
    async fn insert_blocklist_lookup_entry(
        &self,
        blocklist_lookup_entry: storage::BlocklistLookupNew,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
        Ok(storage::BlocklistLookup {
            id: 4,
            merchant_id: "1234".to_string(),
            kms_decrypted_hash: "hash".to_string(),
        })
    }

    async fn find_blocklist_lookup_entry_by_merchant_id_kms_decrypted_hash (
        &self,
        merchant_id: String,
        kms_decrypted_hash: String,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
        Ok(storage::BlocklistLookup {
            id: 4,
            merchant_id: "1234".to_string(),
            kms_decrypted_hash: "hash".to_string(),
        })
    }

    async fn delete_blocklist_lookup_entry_by_merchant_id_kms_decrypted_hash (
        &self,
        merchant_id: String,
        kms_decrypted_hash: String,
    ) -> CustomResult<bool, errors::StorageError> {
        Ok(true)
    }
}

#[async_trait::async_trait]
impl BlocklistLookupInterface for KafkaStore {
    #[instrument(skip_all)]
    async fn insert_blocklist_lookup_entry(
        &self,
        blocklist_lookup_entry: storage::BlocklistLookupNew,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
        Ok(storage::BlocklistLookup {
            id: 4,
            merchant_id: "1234".to_string(),
            kms_decrypted_hash: "hash".to_string(),
        })
    }

    async fn find_blocklist_lookup_entry_by_merchant_id_kms_decrypted_hash (
        &self,
        merchant_id: String,
        kms_decrypted_hash: String,
    ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
        Ok(storage::BlocklistLookup {
            id: 4,
            merchant_id: "1234".to_string(),
            kms_decrypted_hash: "hash".to_string(),
        })
    }

    async fn delete_blocklist_lookup_entry_by_merchant_id_kms_decrypted_hash (
        &self,
        merchant_id: String,
        kms_decrypted_hash: String,
    ) -> CustomResult<bool, errors::StorageError> {
        Ok(true)
    }
}
