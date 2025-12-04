use common_enums as storage_enums;
use common_utils::{id_type, pii, types::MinorUnit};
use serde::{Deserialize, Serialize};
use storage_enums::MerchantStorageScheme;
use time::PrimitiveDateTime;

use super::payout_attempt::PayoutAttempt;
#[cfg(feature = "olap")]
use super::PayoutFetchConstraints;

#[async_trait::async_trait]
pub trait PayoutsInterface {
    type Error;
    async fn insert_payout(
        &self,
        _payout: PayoutsNew,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Payouts, Self::Error>;

    async fn find_payout_by_merchant_id_payout_id(
        &self,
        _merchant_id: &id_type::MerchantId,
        _payout_id: &id_type::PayoutId,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Payouts, Self::Error>;

    async fn update_payout(
        &self,
        _this: &Payouts,
        _payout: PayoutsUpdate,
        _payout_attempt: &PayoutAttempt,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Payouts, Self::Error>;

    async fn find_optional_payout_by_merchant_id_payout_id(
        &self,
        _merchant_id: &id_type::MerchantId,
        _payout_id: &id_type::PayoutId,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Option<Payouts>, Self::Error>;

    #[cfg(feature = "olap")]
    async fn filter_payouts_by_constraints(
        &self,
        _merchant_id: &id_type::MerchantId,
        _filters: &PayoutFetchConstraints,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<Payouts>, Self::Error>;

    #[cfg(feature = "olap")]
    async fn filter_payouts_and_attempts(
        &self,
        _merchant_id: &id_type::MerchantId,
        _filters: &PayoutFetchConstraints,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<
        Vec<(
            Payouts,
            PayoutAttempt,
            Option<diesel_models::Customer>,
            Option<diesel_models::Address>,
        )>,
        Self::Error,
    >;

    #[cfg(feature = "olap")]
    async fn filter_payouts_by_time_range_constraints(
        &self,
        _merchant_id: &id_type::MerchantId,
        _time_range: &common_utils::types::TimeRange,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<Payouts>, Self::Error>;

    #[cfg(feature = "olap")]
    #[allow(clippy::too_many_arguments)]
    async fn get_total_count_of_filtered_payouts(
        &self,
        _merchant_id: &id_type::MerchantId,
        _active_payout_ids: &[id_type::PayoutId],
        _connector: Option<Vec<api_models::enums::PayoutConnectors>>,
        _currency: Option<Vec<storage_enums::Currency>>,
        _status: Option<Vec<storage_enums::PayoutStatus>>,
        _payout_method: Option<Vec<storage_enums::PayoutType>>,
    ) -> error_stack::Result<i64, Self::Error>;

    #[cfg(feature = "olap")]
    async fn filter_active_payout_ids_by_constraints(
        &self,
        _merchant_id: &id_type::MerchantId,
        _constraints: &PayoutFetchConstraints,
    ) -> error_stack::Result<Vec<id_type::PayoutId>, Self::Error>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Payouts {
    pub payout_id: id_type::PayoutId,
    pub merchant_id: id_type::MerchantId,
    pub customer_id: Option<id_type::CustomerId>,
    pub address_id: Option<String>,
    pub payout_type: Option<storage_enums::PayoutType>,
    pub payout_method_id: Option<String>,
    pub amount: MinorUnit,
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
    pub profile_id: id_type::ProfileId,
    pub status: storage_enums::PayoutStatus,
    pub confirm: Option<bool>,
    pub payout_link_id: Option<String>,
    pub client_secret: Option<String>,
    pub priority: Option<storage_enums::PayoutSendPriority>,
    pub organization_id: Option<id_type::OrganizationId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutsNew {
    pub payout_id: id_type::PayoutId,
    pub merchant_id: id_type::MerchantId,
    pub customer_id: Option<id_type::CustomerId>,
    pub address_id: Option<String>,
    pub payout_type: Option<storage_enums::PayoutType>,
    pub payout_method_id: Option<String>,
    pub amount: MinorUnit,
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
    pub profile_id: id_type::ProfileId,
    pub status: storage_enums::PayoutStatus,
    pub confirm: Option<bool>,
    pub payout_link_id: Option<String>,
    pub client_secret: Option<String>,
    pub priority: Option<storage_enums::PayoutSendPriority>,
    pub organization_id: Option<id_type::OrganizationId>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PayoutsUpdate {
    Update {
        amount: MinorUnit,
        destination_currency: storage_enums::Currency,
        source_currency: storage_enums::Currency,
        description: Option<String>,
        recurring: bool,
        auto_fulfill: bool,
        return_url: Option<String>,
        entity_type: storage_enums::PayoutEntityType,
        metadata: Option<pii::SecretSerdeValue>,
        profile_id: Option<id_type::ProfileId>,
        status: Option<storage_enums::PayoutStatus>,
        confirm: Option<bool>,
        payout_type: Option<storage_enums::PayoutType>,
        address_id: Option<String>,
        customer_id: Option<id_type::CustomerId>,
    },
    PayoutMethodIdUpdate {
        payout_method_id: String,
    },
    RecurringUpdate {
        recurring: bool,
    },
    AttemptCountUpdate {
        attempt_count: i16,
    },
    StatusUpdate {
        status: storage_enums::PayoutStatus,
    },
}

#[derive(Clone, Debug, Default)]
pub struct PayoutsUpdateInternal {
    pub amount: Option<MinorUnit>,
    pub destination_currency: Option<storage_enums::Currency>,
    pub source_currency: Option<storage_enums::Currency>,
    pub description: Option<String>,
    pub recurring: Option<bool>,
    pub auto_fulfill: Option<bool>,
    pub return_url: Option<String>,
    pub entity_type: Option<storage_enums::PayoutEntityType>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub payout_method_id: Option<String>,
    pub profile_id: Option<id_type::ProfileId>,
    pub status: Option<storage_enums::PayoutStatus>,
    pub attempt_count: Option<i16>,
    pub confirm: Option<bool>,
    pub payout_type: Option<common_enums::PayoutType>,
    pub address_id: Option<String>,
    pub customer_id: Option<id_type::CustomerId>,
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
                confirm,
                payout_type,
                address_id,
                customer_id,
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
                confirm,
                payout_type,
                address_id,
                customer_id,
                ..Default::default()
            },
            PayoutsUpdate::PayoutMethodIdUpdate { payout_method_id } => Self {
                payout_method_id: Some(payout_method_id),
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
            PayoutsUpdate::StatusUpdate { status } => Self {
                status: Some(status),
                ..Default::default()
            },
        }
    }
}
