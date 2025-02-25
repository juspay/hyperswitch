// use error_stack::report;
// use router_env::{instrument, tracing};

// use crate::{
//     connection,
//     core::errors::{self, CustomResult},
//     db::MockDb,
//     services::Store,
//     types::storage,
// };

// use hyperswitch_domain_models::errors;
use common_utils::errors::CustomResult;
use diesel_models::generic_link as storage;

#[async_trait::async_trait]
#[allow(dead_code)]
pub trait GenericLinkInterface {
    type Error;
    async fn find_generic_link_by_link_id(
        &self,
        link_id: &str,
    ) -> CustomResult<storage::GenericLinkState, Self::Error>;

    async fn find_pm_collect_link_by_link_id(
        &self,
        link_id: &str,
    ) -> CustomResult<storage::PaymentMethodCollectLink, Self::Error>;

    async fn find_payout_link_by_link_id(
        &self,
        link_id: &str,
    ) -> CustomResult<storage::PayoutLink, Self::Error>;

    async fn insert_generic_link(
        &self,
        _generic_link: storage::GenericLinkNew,
    ) -> CustomResult<storage::GenericLinkState, Self::Error>;

    async fn insert_pm_collect_link(
        &self,
        _pm_collect_link: storage::GenericLinkNew,
    ) -> CustomResult<storage::PaymentMethodCollectLink, Self::Error>;

    async fn insert_payout_link(
        &self,
        _payout_link: storage::GenericLinkNew,
    ) -> CustomResult<storage::PayoutLink, Self::Error>;

    async fn update_payout_link(
        &self,
        payout_link: storage::PayoutLink,
        payout_link_update: storage::PayoutLinkUpdate,
    ) -> CustomResult<storage::PayoutLink, Self::Error>;
}

// #[async_trait::async_trait]
// impl GenericLinkInterface for MockDb {
//     async fn find_generic_link_by_link_id(
//         &self,
//         _generic_link_id: &str,
//     ) -> CustomResult<storage::GenericLinkState, errors::StorageError> {
//         // TODO: Implement function for `MockDb`x
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn find_pm_collect_link_by_link_id(
//         &self,
//         _generic_link_id: &str,
//     ) -> CustomResult<storage::PaymentMethodCollectLink, errors::StorageError> {
//         // TODO: Implement function for `MockDb`x
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn find_payout_link_by_link_id(
//         &self,
//         _generic_link_id: &str,
//     ) -> CustomResult<storage::PayoutLink, errors::StorageError> {
//         // TODO: Implement function for `MockDb`x
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn insert_generic_link(
//         &self,
//         _generic_link: storage::GenericLinkNew,
//     ) -> CustomResult<storage::GenericLinkState, errors::StorageError> {
//         // TODO: Implement function for `MockDb`
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn insert_pm_collect_link(
//         &self,
//         _pm_collect_link: storage::GenericLinkNew,
//     ) -> CustomResult<storage::PaymentMethodCollectLink, errors::StorageError> {
//         // TODO: Implement function for `MockDb`
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn insert_payout_link(
//         &self,
//         _pm_collect_link: storage::GenericLinkNew,
//     ) -> CustomResult<storage::PayoutLink, errors::StorageError> {
//         // TODO: Implement function for `MockDb`
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn update_payout_link(
//         &self,
//         _payout_link: storage::PayoutLink,
//         _payout_link_update: storage::PayoutLinkUpdate,
//     ) -> CustomResult<storage::PayoutLink, errors::StorageError> {
//         // TODO: Implement function for `MockDb`
//         Err(errors::StorageError::MockDbError)?
//     }
// }
