use common_utils::pii;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{self, Deserialize, Serialize};
use serde_json::{self};
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
    pub payout_method_data: Option<serde_json::Value>,
    pub amount: i64,
    pub destination_currency: storage_enums::Currency,
    pub source_currency: storage_enums::Currency,
    pub description: Option<String>,
    pub recurring: bool,
    pub auto_fulfill: bool,
    pub return_url: Option<String>,
    pub entity_type: storage_enums::EntityType,
    pub metadata: Option<pii::SecretSerdeValue>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: PrimitiveDateTime,
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
            payout_method_data: Some(serde_json::Value::Null),
            amount: i64::default(),
            destination_currency: storage_enums::Currency::default(),
            source_currency: storage_enums::Currency::default(),
            description: Option::default(),
            recurring: bool::default(),
            auto_fulfill: bool::default(),
            return_url: Some("https://www.google.com".to_string()),
            entity_type: storage_enums::EntityType::default(),
            metadata: Option::default(),
            created_at: now,
            last_modified_at: now,
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
    pub payout_method_data: Option<serde_json::Value>,
    pub amount: i64,
    pub destination_currency: storage_enums::Currency,
    pub source_currency: storage_enums::Currency,
    pub description: String,
    pub recurring: bool,
    pub auto_fulfill: bool,
    pub return_url: Option<String>,
    pub entity_type: storage_enums::EntityType,
    pub metadata: Option<pii::SecretSerdeValue>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub last_modified_at: Option<PrimitiveDateTime>,
}

#[derive(Debug)]
pub enum PayoutCreateUpdate {
    RecurringUpdate { recurring: bool },
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = payout_create)]
pub struct PayoutCreateUpdateInternal {
    pub recurring: Option<bool>,
}

impl From<PayoutCreateUpdate> for PayoutCreateUpdateInternal {
    fn from(payout_update: PayoutCreateUpdate) -> Self {
        match payout_update {
            PayoutCreateUpdate::RecurringUpdate { recurring } => Self {
                recurring: Some(recurring),
            },
        }
    }
}
