use common_enums as storage_enums;
use common_utils::pii;
use serde::{Deserialize, Serialize};
use storage_enums::MerchantStorageScheme;
use time::PrimitiveDateTime;

#[cfg(feature = "olap")]
use super::{payout_attempt::PayoutAttempt, PayoutFetchConstraints};
use crate::errors;

#[async_trait::async_trait]
pub trait PayoutsInterface {
    async fn insert_payout(
        &self,
        _payout: PayoutsNew,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Payouts, errors::StorageError>;

    async fn find_payout_by_merchant_id_payout_id(
        &self,
        _merchant_id: &str,
        _payout_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Payouts, errors::StorageError>;

    async fn update_payout(
        &self,
        _this: &Payouts,
        _payout: PayoutsUpdate,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Payouts, errors::StorageError>;

    #[cfg(feature = "olap")]
    async fn filter_payouts_by_constraints(
        &self,
        _merchant_id: &str,
        _filters: &PayoutFetchConstraints,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<Payouts>, errors::StorageError>;

    #[cfg(feature = "olap")]
    async fn filter_payouts_and_attempts(
        &self,
        _merchant_id: &str,
        _filters: &PayoutFetchConstraints,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<(Payouts, PayoutAttempt)>, errors::StorageError>;

    #[cfg(feature = "olap")]
    async fn filter_payouts_by_time_range_constraints(
        &self,
        merchant_id: &str,
        time_range: &api_models::payments::TimeRange,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<Payouts>, errors::StorageError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Payouts {
    pub payout_id: String,
    pub merchant_id: String,
    pub customer_id: String,
    pub address_id: String,
    pub payout_type: storage_enums::PayoutType,
    pub payout_method_id: Option<String>,
    pub amount: i64,
    pub destination_currency: storage_enums::Currency,
    pub source_currency: storage_enums::Currency,
    pub description: Option<String>,
    pub recurring: bool,
    pub auto_fulfill: bool,
    pub return_url: Option<String>,
    pub entity_type: storage_enums::PayoutEntityType,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub attempt_count: i16,
    pub profile_id: String,
    pub status: storage_enums::PayoutStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutsNew {
    pub payout_id: String,
    pub merchant_id: String,
    pub customer_id: String,
    pub address_id: String,
    pub payout_type: storage_enums::PayoutType,
    pub payout_method_id: Option<String>,
    pub amount: i64,
    pub destination_currency: storage_enums::Currency,
    pub source_currency: storage_enums::Currency,
    pub description: Option<String>,
    pub recurring: bool,
    pub auto_fulfill: bool,
    pub return_url: Option<String>,
    pub entity_type: storage_enums::PayoutEntityType,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub created_at: Option<PrimitiveDateTime>,
    pub last_modified_at: Option<PrimitiveDateTime>,
    pub profile_id: String,
    pub status: storage_enums::PayoutStatus,
    pub attempt_count: i16,
}

impl Default for PayoutsNew {
    fn default() -> Self {
        let now = common_utils::date_time::now();

        Self {
            payout_id: String::default(),
            merchant_id: String::default(),
            customer_id: String::default(),
            address_id: String::default(),
            payout_type: storage_enums::PayoutType::default(),
            payout_method_id: Option::default(),
            amount: i64::default(),
            destination_currency: storage_enums::Currency::default(),
            source_currency: storage_enums::Currency::default(),
            description: Option::default(),
            recurring: bool::default(),
            auto_fulfill: bool::default(),
            return_url: None,
            entity_type: storage_enums::PayoutEntityType::default(),
            metadata: Option::default(),
            created_at: Some(now),
            last_modified_at: Some(now),
            profile_id: String::default(),
            status: storage_enums::PayoutStatus::default(),
            attempt_count: 1,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PayoutsUpdate {
    Update {
        amount: i64,
        destination_currency: storage_enums::Currency,
        source_currency: storage_enums::Currency,
        description: Option<String>,
        recurring: bool,
        auto_fulfill: bool,
        return_url: Option<String>,
        entity_type: storage_enums::PayoutEntityType,
        metadata: Option<pii::SecretSerdeValue>,
        profile_id: Option<String>,
        status: Option<storage_enums::PayoutStatus>,
    },
    PayoutMethodIdUpdate {
        payout_method_id: Option<String>,
    },
    RecurringUpdate {
        recurring: bool,
    },
    AttemptCountUpdate {
        attempt_count: i16,
    },
}

#[derive(Clone, Debug, Default)]
pub struct PayoutsUpdateInternal {
    pub amount: Option<i64>,
    pub destination_currency: Option<storage_enums::Currency>,
    pub source_currency: Option<storage_enums::Currency>,
    pub description: Option<String>,
    pub recurring: Option<bool>,
    pub auto_fulfill: Option<bool>,
    pub return_url: Option<String>,
    pub entity_type: Option<storage_enums::PayoutEntityType>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub payout_method_id: Option<String>,
    pub profile_id: Option<String>,
    pub status: Option<storage_enums::PayoutStatus>,
    pub attempt_count: Option<i16>,
}

impl From<PayoutsUpdate> for PayoutsUpdateInternal {
    fn from(payout_update: PayoutsUpdate) -> Self {
        match payout_update {
            PayoutsUpdate::Update {
                amount,
                destination_currency,
                source_currency,
                description,
                recurring,
                auto_fulfill,
                return_url,
                entity_type,
                metadata,
                profile_id,
                status,
            } => Self {
                amount: Some(amount),
                destination_currency: Some(destination_currency),
                source_currency: Some(source_currency),
                description,
                recurring: Some(recurring),
                auto_fulfill: Some(auto_fulfill),
                return_url,
                entity_type: Some(entity_type),
                metadata,
                profile_id,
                status,
                ..Default::default()
            },
            PayoutsUpdate::PayoutMethodIdUpdate { payout_method_id } => Self {
                payout_method_id,
                ..Default::default()
            },
            PayoutsUpdate::RecurringUpdate { recurring } => Self {
                recurring: Some(recurring),
                ..Default::default()
            },
            PayoutsUpdate::AttemptCountUpdate { attempt_count } => Self {
                attempt_count: Some(attempt_count),
                ..Default::default()
            },
        }
    }
}
