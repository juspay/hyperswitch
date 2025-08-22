use common_utils::errors::CustomResult;
use diesel_models::enums as storage_enums;
use hyperswitch_domain_models::payouts::{
    payout_attempt::PayoutAttempt,
    payouts::{Payouts, PayoutsInterface, PayoutsNew, PayoutsUpdate},
};

use crate::{errors::StorageError, MockDb};

#[async_trait::async_trait]
impl PayoutsInterface for MockDb {
    type Error = StorageError;
    async fn find_payout_by_merchant_id_payout_id(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        _payout_id: &common_utils::id_type::PayoutId,
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
        _merchant_id: &common_utils::id_type::MerchantId,
        _payout_id: &common_utils::id_type::PayoutId,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<Option<Payouts>, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(feature = "olap")]
    async fn filter_payouts_by_constraints(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        _filters: &hyperswitch_domain_models::payouts::PayoutFetchConstraints,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<Payouts>, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(feature = "olap")]
    async fn filter_payouts_and_attempts(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        _filters: &hyperswitch_domain_models::payouts::PayoutFetchConstraints,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<
        Vec<(
            Payouts,
            PayoutAttempt,
            Option<diesel_models::Customer>,
            Option<diesel_models::Address>,
        )>,
        StorageError,
    > {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(feature = "olap")]
    async fn filter_payouts_by_time_range_constraints(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        _time_range: &common_utils::types::TimeRange,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<Payouts>, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(feature = "olap")]
    async fn get_total_count_of_filtered_payouts(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        _active_payout_ids: &[common_utils::id_type::PayoutId],
        _connector: Option<Vec<api_models::enums::PayoutConnectors>>,
        _currency: Option<Vec<storage_enums::Currency>>,
        _status: Option<Vec<storage_enums::PayoutStatus>>,
        _payout_method: Option<Vec<storage_enums::PayoutType>>,
    ) -> CustomResult<i64, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(feature = "olap")]
    async fn filter_active_payout_ids_by_constraints(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        _constraints: &hyperswitch_domain_models::payouts::PayoutFetchConstraints,
    ) -> CustomResult<Vec<common_utils::id_type::PayoutId>, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }
}
