// use error_stack::report;
// use router_env::{instrument, tracing};

// use crate::{
//     connection,
//     core::errors::{self, CustomResult},
//     db::MockDb,
//     services::Store,
//     types::storage::{self, PaymentLinkDbExt},
// };

// use hyperswitch_domain_models::errors;
use common_utils::errors::CustomResult;
use diesel_models::payment_link as storage;

#[async_trait::async_trait]
#[allow(dead_code)]
pub trait PaymentLinkInterface {
    type Error;
    async fn find_payment_link_by_payment_link_id(
        &self,
        payment_link_id: &str,
    ) -> CustomResult<storage::PaymentLink, Self::Error>;

    async fn insert_payment_link(
        &self,
        _payment_link: storage::PaymentLinkNew,
    ) -> CustomResult<storage::PaymentLink, Self::Error>;

    // (TODO:jarnura) api model used in storage
    // async fn list_payment_link_by_merchant_id(
    //     &self,
    //     merchant_id: &common_utils::id_type::MerchantId,
    //     payment_link_constraints: api_models::payments::PaymentLinkListConstraints,
    // ) -> CustomResult<Vec<storage::PaymentLink>, Self::Error>;
}

// #[async_trait::async_trait]
// impl PaymentLinkInterface for MockDb {
//     async fn insert_payment_link(
//         &self,
//         _payment_link: storage::PaymentLinkNew,
//     ) -> CustomResult<storage::PaymentLink, errors::StorageError> {
//         // TODO: Implement function for `MockDb`
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn find_payment_link_by_payment_link_id(
//         &self,
//         _payment_link_id: &str,
//     ) -> CustomResult<storage::PaymentLink, errors::StorageError> {
//         // TODO: Implement function for `MockDb`x
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn list_payment_link_by_merchant_id(
//         &self,
//         _merchant_id: &common_utils::id_type::MerchantId,
//         _payment_link_constraints: api_models::payments::PaymentLinkListConstraints,
//     ) -> CustomResult<Vec<storage::PaymentLink>, errors::StorageError> {
//         // TODO: Implement function for `MockDb`x
//         Err(errors::StorageError::MockDbError)?
//     }
// }
