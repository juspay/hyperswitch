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
use diesel_models::blocklist_lookup as storage;

#[async_trait::async_trait]
#[allow(dead_code)]
pub trait BlocklistLookupInterface {
    type Error;
    async fn insert_blocklist_lookup_entry(
        &self,
        blocklist_lookup_new: storage::BlocklistLookupNew,
    ) -> CustomResult<storage::BlocklistLookup, Self::Error>;

    async fn find_blocklist_lookup_entry_by_merchant_id_fingerprint(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        fingerprint: &str,
    ) -> CustomResult<storage::BlocklistLookup, Self::Error>;

    async fn delete_blocklist_lookup_entry_by_merchant_id_fingerprint(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        fingerprint: &str,
    ) -> CustomResult<storage::BlocklistLookup, Self::Error>;
}

// #[async_trait::async_trait]
// impl BlocklistLookupInterface for MockDb {
//     #[instrument(skip_all)]
//     async fn insert_blocklist_lookup_entry(
//         &self,
//         _blocklist_lookup_entry: storage::BlocklistLookupNew,
//     ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn find_blocklist_lookup_entry_by_merchant_id_fingerprint(
//         &self,
//         _merchant_id: &common_utils::id_type::MerchantId,
//         _fingerprint: &str,
//     ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn delete_blocklist_lookup_entry_by_merchant_id_fingerprint(
//         &self,
//         _merchant_id: &common_utils::id_type::MerchantId,
//         _fingerprint: &str,
//     ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
//         Err(errors::StorageError::MockDbError)?
//     }
// }

// #[async_trait::async_trait]
// impl BlocklistLookupInterface for KafkaStore {
//     #[instrument(skip_all)]
//     async fn insert_blocklist_lookup_entry(
//         &self,
//         blocklist_lookup_entry: storage::BlocklistLookupNew,
//     ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
//         self.diesel_store
//             .insert_blocklist_lookup_entry(blocklist_lookup_entry)
//             .await
//     }

//     #[instrument(skip_all)]
//     async fn find_blocklist_lookup_entry_by_merchant_id_fingerprint(
//         &self,
//         merchant_id: &common_utils::id_type::MerchantId,
//         fingerprint: &str,
//     ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
//         self.diesel_store
//             .find_blocklist_lookup_entry_by_merchant_id_fingerprint(merchant_id, fingerprint)
//             .await
//     }

//     #[instrument(skip_all)]
//     async fn delete_blocklist_lookup_entry_by_merchant_id_fingerprint(
//         &self,
//         merchant_id: &common_utils::id_type::MerchantId,
//         fingerprint: &str,
//     ) -> CustomResult<storage::BlocklistLookup, errors::StorageError> {
//         self.diesel_store
//             .delete_blocklist_lookup_entry_by_merchant_id_fingerprint(merchant_id, fingerprint)
//             .await
//     }
// }
