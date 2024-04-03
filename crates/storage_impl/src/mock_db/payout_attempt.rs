use common_utils::errors::CustomResult;
use data_models::{
    errors::StorageError,
    payouts::payout_attempt::{
        PayoutAttempt, PayoutAttemptInterface, PayoutAttemptNew, PayoutAttemptUpdate,
    },
};
use diesel_models::enums as storage_enums;

use super::MockDb;

#[async_trait::async_trait]
impl PayoutAttemptInterface for MockDb {
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

    async fn get_filters_for_payouts(
        &self,
        _payouts: &[data_models::payouts::payouts::Payouts],
        _merchant_id: &str,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<data_models::payouts::payout_attempt::PayoutListFilters, StorageError> {
        Err(StorageError::MockDbError)?
    }
}
