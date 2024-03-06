use common_utils::errors::CustomResult;
#[cfg(feature = "payouts")]
use data_models::payouts::payouts::PayoutsInterface;
#[cfg(not(feature = "payouts"))]
use data_models::PayoutsInterface;
use data_models::{
    errors::StorageError,
    payouts::payouts::{Payouts, PayoutsNew, PayoutsUpdate},
};
use diesel_models::enums as storage_enums;

use super::MockDb;

#[cfg(not(feature = "payouts"))]
impl PayoutsInterface for MockDb {}

#[cfg(feature = "payouts")]
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
}
