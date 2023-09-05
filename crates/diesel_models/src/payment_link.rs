
use diesel::{Identifiable, Insertable, Queryable};
use serde::{self, Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::payment_link};

#[derive(Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Serialize, Deserialize)]
#[diesel(table_name = payment_link)]
#[diesel(primary_key(payment_id))]
pub struct PaymentLink {
    pub id: i32,
    pub payment_id: String,
    pub merchant_id: String,
    pub link_to_pay: String,
    pub amount : i64,
    pub currency: storage_enums::Currency,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: PrimitiveDateTime,
}

impl Default for PaymentLink {
    fn default() -> Self {
        let now = common_utils::date_time::now();

        Self {
            id: i32::default(),
            payment_id: String::default(),
            merchant_id: String::default(),
            link_to_pay: String::default(),
            amount: i64::default(),
            currency: storage_enums::Currency::default(),
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
    Serialize,
    Deserialize,
)]
#[diesel(table_name = payment_link)]
pub struct PaymentLinkNew {
    pub payment_id: String,
    pub merchant_id: String,
    pub link_to_pay: String,
    pub amount: i64,
    pub currency: storage_enums::Currency,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_modified_at: Option<PrimitiveDateTime>
}
