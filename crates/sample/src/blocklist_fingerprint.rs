// use error_stack::report;
// use router_env::{instrument, tracing};
// use storage_impl::MockDb;

// use super::Store;
// use crate::{
//     connection,
//     core::errors::{self, CustomResult},
//     db::kafka_store::KafkaStore,
//     types::storage,
// };

// use hyperswitch_domain_models::errors;
use common_utils::errors::CustomResult;
use diesel_models::blocklist_fingerprint as storage;

#[async_trait::async_trait]
#[allow(dead_code)]
pub trait BlocklistFingerprintInterface {
    type Error;
    async fn insert_blocklist_fingerprint_entry(
        &self,
        pm_fingerprint_new: storage::BlocklistFingerprintNew,
    ) -> CustomResult<storage::BlocklistFingerprint, Self::Error>;

    async fn find_blocklist_fingerprint_by_merchant_id_fingerprint_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> CustomResult<storage::BlocklistFingerprint, Self::Error>;
}

// #[async_trait::async_trait]
// impl BlocklistFingerprintInterface for MockDb {
//     async fn insert_blocklist_fingerprint_entry(
//         &self,
//         _pm_fingerprint_new: storage::BlocklistFingerprintNew,
//     ) -> CustomResult<storage::BlocklistFingerprint, errors::StorageError> {
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn find_blocklist_fingerprint_by_merchant_id_fingerprint_id(
//         &self,
//         _merchant_id: &common_utils::id_type::MerchantId,
//         _fingerprint_id: &str,
//     ) -> CustomResult<storage::BlocklistFingerprint, errors::StorageError> {
//         Err(errors::StorageError::MockDbError)?
//     }
// }

// #[async_trait::async_trait]
// impl BlocklistFingerprintInterface for KafkaStore {
//     #[instrument(skip_all)]
//     async fn insert_blocklist_fingerprint_entry(
//         &self,
//         pm_fingerprint_new: storage::BlocklistFingerprintNew,
//     ) -> CustomResult<storage::BlocklistFingerprint, errors::StorageError> {
//         self.diesel_store
//             .insert_blocklist_fingerprint_entry(pm_fingerprint_new)
//             .await
//     }

//     #[instrument(skip_all)]
//     async fn find_blocklist_fingerprint_by_merchant_id_fingerprint_id(
//         &self,
//         merchant_id: &common_utils::id_type::MerchantId,
//         fingerprint: &str,
//     ) -> CustomResult<storage::BlocklistFingerprint, errors::StorageError> {
//         self.diesel_store
//             .find_blocklist_fingerprint_by_merchant_id_fingerprint_id(merchant_id, fingerprint)
//             .await
//     }
// }
