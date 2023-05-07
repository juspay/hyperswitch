use common_utils::pii;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{self, Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::payout_create};

#[derive(Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Serialize, Deserialize)]
#[diesel(table_name = payout_create)]
pub struct PayoutCreate {
    pub id: i32,
    pub payout_id: String,
    pub merchant_id: String,
    pub customer_id: String,
    pub address_id: String,
    pub payout_type: storage_enums::PayoutType,
    pub amount: i64,
    pub destination_currency: storage_enums::Currency,
    pub source_currency: storage_enums::Currency,
    pub description: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    pub status: storage_enums::PayoutStatus,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub recurring: bool,
    pub connector: String,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
}

impl Default for PayoutCreate {
    fn default() -> Self {
        let now = common_utils::date_time::now();

        Self {
            id: i32::default(),
            payout_id: String::default(),
            merchant_id: String::default(),
            customer_id: String::default(),
            address_id: String::default(),
            payout_type: storage_enums::PayoutType::default(),
            amount: i64::default(),
            destination_currency: storage_enums::Currency::default(),
            source_currency: storage_enums::Currency::default(),
            description: Option::default(),
            created_at: now,
            modified_at: now,
            status: storage_enums::PayoutStatus::default(),
            metadata: Option::default(),
            recurring: bool::default(),
            connector: String::default(),
            error_message: Option::default(),
            error_code: Option::default(),
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
    router_derive::DebugAsDisplay,
    serde::Serialize,
    serde::Deserialize,
    router_derive::Setter,
)]
#[diesel(table_name = payout_create)]
pub struct PayoutCreateNew {
    pub payout_id: String,
    pub merchant_id: String,
    pub customer_id: String,
    pub address_id: String,
    pub payout_type: storage_enums::PayoutType,
    pub amount: i64,
    pub destination_currency: storage_enums::Currency,
    pub source_currency: storage_enums::Currency,
    pub description: String,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub modified_at: Option<PrimitiveDateTime>,
    pub status: storage_enums::PayoutStatus,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub recurring: bool,
    pub connector: String,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
}

#[derive(Debug)]
pub enum PayoutCreateUpdate {
    CreationUpdate {
        status: storage_enums::PayoutStatus,
        error_message: Option<String>,
        error_code: Option<String>,
    },
    RecurringUpdate {
        recurring: bool,
    },
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = payout_create)]
pub struct PayoutCreateUpdateInternal {
    pub status: Option<storage_enums::PayoutStatus>,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub recurring: Option<bool>,
}

impl From<PayoutCreateUpdate> for PayoutCreateUpdateInternal {
    fn from(payout_update: PayoutCreateUpdate) -> Self {
        match payout_update {
            PayoutCreateUpdate::CreationUpdate {
                status,
                error_message,
                error_code,
            } => Self {
                status: Some(status),
                error_message,
                error_code,
                recurring: None,
            },
            PayoutCreateUpdate::RecurringUpdate { recurring } => Self {
                recurring: Some(recurring),
                status: None,
                error_message: None,
                error_code: None,
            },
        }
    }
}
