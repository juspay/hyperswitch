use common_utils::errors::CustomResult;
use diesel_models::enums as storage_enums;
use hyperswitch_domain_models::{
    errors::StorageError,
    payouts::{
        payout_attempt::PayoutAttempt,
        payouts::{Payouts, PayoutsInterface, PayoutsNew, PayoutsUpdate},
    },
};

use super::MockDb;

#[async_trait::async_trait]
impl PayoutsInterface for MockDb {
    async fn find_payout_by_merchant_id_payout_id(
        &self,
        _merchant_id: &str,
        _payout_id: &str,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<Payouts, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    async fn update_payout(
        &self,
        _this: &Payouts,
        _payout_update: PayoutsUpdate,
        _payout_attempt: &PayoutAttempt,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<Payouts, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    async fn insert_payout(
        &self,
        _payout: PayoutsNew,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<Payouts, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    async fn find_optional_payout_by_merchant_id_payout_id(
        &self,
        _merchant_id: &str,
        _payout_id: &str,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<Option<Payouts>, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(feature = "olap")]
    async fn filter_payouts_by_constraints(
        &self,
        _merchant_id: &str,
        _filters: &hyperswitch_domain_models::payouts::PayoutFetchConstraints,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<Payouts>, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(feature = "olap")]
    async fn filter_payouts_and_attempts(
        &self,
        _merchant_id: &str,
        _filters: &hyperswitch_domain_models::payouts::PayoutFetchConstraints,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<(Payouts, PayoutAttempt, diesel_models::Customer)>, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(feature = "olap")]
    async fn filter_payouts_by_time_range_constraints(
        &self,
        _merchant_id: &str,
        _time_range: &api_models::payments::TimeRange,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<Payouts>, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }
}
