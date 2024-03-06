use common_utils::errors::CustomResult;
#[cfg(feature = "payouts")]
use data_models::payouts::payout_attempt::PayoutAttemptInterface;
#[cfg(not(feature = "payouts"))]
use data_models::PayoutAttemptInterface;
use data_models::{
    errors::StorageError,
    payouts::payout_attempt::{PayoutAttempt, PayoutAttemptNew, PayoutAttemptUpdate},
};
use diesel_models::enums as storage_enums;

use super::MockDb;

#[cfg(not(feature = "payouts"))]
impl PayoutAttemptInterface for MockDb {}

#[cfg(feature = "payouts")]
#[async_trait::async_trait]
impl PayoutAttemptInterface for MockDb {
    async fn find_payout_attempt_by_merchant_id_payout_id(
        &self,
        _merchant_id: &str,
        _payout_id: &str,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PayoutAttempt, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    async fn update_payout_attempt(
        &self,
        _this: &PayoutAttempt,
        _payout_attempt_update: PayoutAttemptUpdate,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PayoutAttempt, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    async fn insert_payout_attempt(
        &self,
        _payout: PayoutAttemptNew,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PayoutAttempt, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    async fn find_payout_attempt_by_merchant_id_payout_attempt_id(
        &self,
        _merchant_id: &str,
        _payout_attempt_id: &str,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PayoutAttempt, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }
}
