use diesel::{Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{
    enums as storage_enums,
    schema::{payout_create, payouts},
};

#[derive(Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Serialize, Deserialize)]
#[diesel(table_name = payout_create)]
pub struct PayoutCreate {
    pub id: i32,
    pub payout_id: String,
    pub customer_id: String,
    pub merchant_id: String,
    pub status: storage_enums::PayoutStatus,
    pub created_at: PrimitiveDateTime,
    pub encoded_data: Option<serde_json::Value>,
    pub connector: String,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Insertable, Queryable, router_derive::DebugAsDisplay)]
#[diesel(table_name = payout_create)]
pub struct PayoutCreateNew {
    pub payout_id: String,
    pub customer_id: String,
    pub merchant_id: String,
    pub status: storage_enums::PayoutStatus,
    pub created_at: PrimitiveDateTime,
    pub encoded_data: Option<serde_json::Value>,
    pub connector: String,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
}

impl Default for PayoutCreateNew {
    fn default() -> Self {
        let now = common_utils::date_time::now();

        Self {
            payout_id: String::default(),
            customer_id: String::default(),
            merchant_id: String::default(),
            status: storage_enums::PayoutStatus::default(),
            created_at: now,
            encoded_data: Option::default(),
            connector: String::default(),
            error_message: Option::default(),
            error_code: Option::default(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Serialize, Deserialize)]
#[diesel(table_name = payouts)]
pub struct Payouts {
    pub id: i32,
    pub payout_id: String,
    pub customer_id: String,
    pub merchant_id: String,
    pub address_id: String,
    pub payout_type: storage_enums::PayoutType,
    pub connector_payout_id: String,
    pub connector: String,
    pub payout_data: Option<serde_json::Value>,
    pub amount: i64,
    pub destination_currency: storage_enums::Currency,
    pub source_currency: storage_enums::Currency,
    pub recurring: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq, Insertable, Queryable, router_derive::DebugAsDisplay)]
#[diesel(table_name = payouts)]
pub struct PayoutsNew {
    pub payout_id: String,
    pub customer_id: String,
    pub merchant_id: String,
    pub address_id: String,
    pub payout_type: storage_enums::PayoutType,
    pub connector_payout_id: String,
    pub connector: String,
    pub payout_data: Option<serde_json::Value>,
    pub amount: i64,
    pub destination_currency: storage_enums::Currency,
    pub source_currency: storage_enums::Currency,
    pub recurring: Option<bool>,
}

impl Default for PayoutsNew {
    fn default() -> Self {
        // let now = common_utils::date_time::now();

        Self {
            payout_id: String::default(),
            customer_id: String::default(),
            merchant_id: String::default(),
            address_id: String::default(),
            payout_type: storage_enums::PayoutType::default(),
            connector_payout_id: String::default(),
            connector: String::default(),
            payout_data: Option::default(),
            amount: i64::default(),
            destination_currency: storage_enums::Currency::default(),
            source_currency: storage_enums::Currency::default(),
            recurring: Some(false),
        }
    }
}
