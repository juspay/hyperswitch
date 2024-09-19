use common_utils::{pii, types::MinorUnit};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use serde::{self, Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::payouts};

// Payouts
#[derive(
    Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Selectable, Serialize, Deserialize,
)]
#[diesel(table_name = payouts, primary_key(payout_id), check_for_backend(diesel::pg::Pg))]
pub struct Payouts {
    pub payout_id: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub customer_id: Option<common_utils::id_type::CustomerId>,
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
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: PrimitiveDateTime,
    pub attempt_count: i16,
    pub profile_id: common_utils::id_type::ProfileId,
    pub status: storage_enums::PayoutStatus,
    pub confirm: Option<bool>,
    pub payout_link_id: Option<String>,
    pub client_secret: Option<String>,
    pub priority: Option<storage_enums::PayoutSendPriority>,
}

#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    Insertable,
    serde::Serialize,
    serde::Deserialize,
    router_derive::DebugAsDisplay,
    router_derive::Setter,
)]
#[diesel(table_name = payouts)]
pub struct PayoutsNew {
    pub payout_id: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub customer_id: Option<common_utils::id_type::CustomerId>,
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
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: PrimitiveDateTime,
    pub attempt_count: i16,
    pub profile_id: common_utils::id_type::ProfileId,
    pub status: storage_enums::PayoutStatus,
    pub confirm: Option<bool>,
    pub payout_link_id: Option<String>,
    pub client_secret: Option<String>,
    pub priority: Option<storage_enums::PayoutSendPriority>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        profile_id: Option<common_utils::id_type::ProfileId>,
        status: Option<storage_enums::PayoutStatus>,
        confirm: Option<bool>,
        payout_type: Option<storage_enums::PayoutType>,
        address_id: Option<String>,
        customer_id: Option<common_utils::id_type::CustomerId>,
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

#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = payouts)]
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
    pub profile_id: Option<common_utils::id_type::ProfileId>,
    pub status: Option<storage_enums::PayoutStatus>,
    pub last_modified_at: PrimitiveDateTime,
    pub attempt_count: Option<i16>,
    pub confirm: Option<bool>,
    pub payout_type: Option<common_enums::PayoutType>,
    pub address_id: Option<String>,
    pub customer_id: Option<common_utils::id_type::CustomerId>,
}

impl Default for PayoutsUpdateInternal {
    fn default() -> Self {
        Self {
            amount: None,
            destination_currency: None,
            source_currency: None,
            description: None,
            recurring: None,
            auto_fulfill: None,
            return_url: None,
            entity_type: None,
            metadata: None,
            payout_method_id: None,
            profile_id: None,
            status: None,
            last_modified_at: common_utils::date_time::now(),
            attempt_count: None,
            confirm: None,
            payout_type: None,
            address_id: None,
            customer_id: None,
        }
    }
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

impl PayoutsUpdate {
    pub fn apply_changeset(self, source: Payouts) -> Payouts {
        let PayoutsUpdateInternal {
            amount,
            destination_currency,
            source_currency,
            description,
            recurring,
            auto_fulfill,
            return_url,
            entity_type,
            metadata,
            payout_method_id,
            profile_id,
            status,
            last_modified_at,
            attempt_count,
            confirm,
            payout_type,
            address_id,
            customer_id,
        } = self.into();
        Payouts {
            amount: amount.unwrap_or(source.amount),
            destination_currency: destination_currency.unwrap_or(source.destination_currency),
            source_currency: source_currency.unwrap_or(source.source_currency),
            description: description.or(source.description),
            recurring: recurring.unwrap_or(source.recurring),
            auto_fulfill: auto_fulfill.unwrap_or(source.auto_fulfill),
            return_url: return_url.or(source.return_url),
            entity_type: entity_type.unwrap_or(source.entity_type),
            metadata: metadata.or(source.metadata),
            payout_method_id: payout_method_id.or(source.payout_method_id),
            profile_id: profile_id.unwrap_or(source.profile_id),
            status: status.unwrap_or(source.status),
            last_modified_at,
            attempt_count: attempt_count.unwrap_or(source.attempt_count),
            confirm: confirm.or(source.confirm),
            payout_type: payout_type.or(source.payout_type),
            address_id: address_id.or(source.address_id),
            customer_id: customer_id.or(source.customer_id),
            ..source
        }
    }
}
