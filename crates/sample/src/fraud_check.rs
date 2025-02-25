// use diesel_models::fraud_check::{self as storage, FraudCheck, FraudCheckUpdate};
// use error_stack::report;
// use router_env::{instrument, tracing};

// use super::MockDb;
// use crate::{
//     connection,
//     core::errors::{self, CustomResult},
//     services::Store,
// };

// use hyperswitch_domain_models::errors;
use common_utils::errors::CustomResult;
use diesel_models::fraud_check as storage;

#[async_trait::async_trait]
#[allow(dead_code)]
pub trait FraudCheckInterface {
    type Error;
    async fn insert_fraud_check_response(
        &self,
        new: storage::FraudCheckNew,
    ) -> CustomResult<storage::FraudCheck, Self::Error>;

    async fn update_fraud_check_response_with_attempt_id(
        &self,
        this: storage::FraudCheck,
        fraud_check: storage::FraudCheckUpdate,
    ) -> CustomResult<storage::FraudCheck, Self::Error>;

    async fn find_fraud_check_by_payment_id(
        &self,
        payment_id: common_utils::id_type::PaymentId,
        merchant_id: common_utils::id_type::MerchantId,
    ) -> CustomResult<storage::FraudCheck, Self::Error>;

    async fn find_fraud_check_by_payment_id_if_present(
        &self,
        payment_id: common_utils::id_type::PaymentId,
        merchant_id: common_utils::id_type::MerchantId,
    ) -> CustomResult<Option<storage::FraudCheck>, Self::Error>;
}

// #[async_trait::async_trait]
// impl FraudCheckInterface for MockDb {
//     async fn insert_fraud_check_response(
//         &self,
//         _new: storage::FraudCheckNew,
//     ) -> CustomResult<FraudCheck, errors::StorageError> {
//         Err(errors::StorageError::MockDbError)?
//     }
//     async fn update_fraud_check_response_with_attempt_id(
//         &self,
//         _this: FraudCheck,
//         _fraud_check: FraudCheckUpdate,
//     ) -> CustomResult<FraudCheck, errors::StorageError> {
//         Err(errors::StorageError::MockDbError)?
//     }
//     async fn find_fraud_check_by_payment_id(
//         &self,
//         _payment_id: common_utils::id_type::PaymentId,
//         _merchant_id: common_utils::id_type::MerchantId,
//     ) -> CustomResult<FraudCheck, errors::StorageError> {
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn find_fraud_check_by_payment_id_if_present(
//         &self,
//         _payment_id: common_utils::id_type::PaymentId,
//         _merchant_id: common_utils::id_type::MerchantId,
//     ) -> CustomResult<Option<FraudCheck>, errors::StorageError> {
//         Err(errors::StorageError::MockDbError)?
//     }
// }
