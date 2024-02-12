use common_utils::pii;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{self, Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::payouts};

// Payouts
#[derive(Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Serialize, Deserialize)]
#[diesel(table_name = payouts)]
#[diesel(primary_key(payout_id))]
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
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: PrimitiveDateTime,
    pub attempt_count: i16,
}

impl Default for Payouts {
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
            created_at: now,
            last_modified_at: now,
            attempt_count: i16::default(),
        }
    }
}

#[derive(
    Clone,
    Debug,
    Default,
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
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub last_modified_at: Option<PrimitiveDateTime>,
    pub attempt_count: i16,
}

#[derive(Debug)]
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
        last_modified_at: Option<PrimitiveDateTime>,
    },
    PayoutMethodIdUpdate {
        payout_method_id: Option<String>,
        last_modified_at: Option<PrimitiveDateTime>,
    },
    RecurringUpdate {
        recurring: bool,
        last_modified_at: Option<PrimitiveDateTime>,
    },
    AttemptCountUpdate {
        attempt_count: i16,
    },
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = payouts)]
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
    pub last_modified_at: Option<PrimitiveDateTime>,
    pub payout_method_id: Option<String>,
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
                last_modified_at,
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
                last_modified_at,
                ..Default::default()
            },
            PayoutsUpdate::PayoutMethodIdUpdate {
                last_modified_at,
                payout_method_id,
            } => Self {
                last_modified_at,
                payout_method_id,
                ..Default::default()
            },
            PayoutsUpdate::RecurringUpdate {
                last_modified_at,
                recurring,
            } => Self {
                last_modified_at,
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
