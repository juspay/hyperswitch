// use error_stack::report;
// use router_env::{instrument, tracing};

// use super::{MockDb, Store};
// use crate::{
//     connection,
//     core::errors::{self, CustomResult},
//     types::storage,
// };

// use hyperswitch_domain_models::errors;
use common_utils::errors::CustomResult;
use diesel_models::file as storage;

#[async_trait::async_trait]
#[allow(dead_code)]
pub trait FileMetadataInterface {
    type Error;
    async fn insert_file_metadata(
        &self,
        file: storage::FileMetadataNew,
    ) -> CustomResult<storage::FileMetadata, Self::Error>;

    async fn find_file_metadata_by_merchant_id_file_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        file_id: &str,
    ) -> CustomResult<storage::FileMetadata, Self::Error>;

    async fn delete_file_metadata_by_merchant_id_file_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        file_id: &str,
    ) -> CustomResult<bool, Self::Error>;

    async fn update_file_metadata(
        &self,
        this: storage::FileMetadata,
        file_metadata: storage::FileMetadataUpdate,
    ) -> CustomResult<storage::FileMetadata, Self::Error>;
}

// #[async_trait::async_trait]
// impl FileMetadataInterface for MockDb {
//     async fn insert_file_metadata(
//         &self,
//         _file: storage::FileMetadataNew,
//     ) -> CustomResult<storage::FileMetadata, errors::StorageError> {
//         // TODO: Implement function for `MockDb`
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn find_file_metadata_by_merchant_id_file_id(
//         &self,
//         _merchant_id: &common_utils::id_type::MerchantId,
//         _file_id: &str,
//     ) -> CustomResult<storage::FileMetadata, errors::StorageError> {
//         // TODO: Implement function for `MockDb`
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn delete_file_metadata_by_merchant_id_file_id(
//         &self,
//         _merchant_id: &common_utils::id_type::MerchantId,
//         _file_id: &str,
//     ) -> CustomResult<bool, errors::StorageError> {
//         // TODO: Implement function for `MockDb`
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn update_file_metadata(
//         &self,
//         _this: storage::FileMetadata,
//         _file_metadata: storage::FileMetadataUpdate,
//     ) -> CustomResult<storage::FileMetadata, errors::StorageError> {
//         // TODO: Implement function for `MockDb`
//         Err(errors::StorageError::MockDbError)?
//     }
// }
