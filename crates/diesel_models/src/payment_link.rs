use diesel::{Identifiable, Insertable, Queryable};
use serde::{self, Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::payment_link};

#[derive(Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Serialize, Deserialize)]
#[diesel(table_name = payment_link)]
#[diesel(primary_key(payment_id))]
pub struct PaymentLink {
    pub payment_link_id: String,
    pub payment_id: String,
    pub link_to_pay: String,
    pub merchant_id: String,
    pub amount: i64,
    pub currency: Option<storage_enums::Currency>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: PrimitiveDateTime,
    // #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub fullfilment_time: Option<PrimitiveDateTime>,
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
#[diesel(table_name = payment_link)]
pub struct PaymentLinkNew {
    pub payment_link_id: String,
    pub payment_id: String,
    pub link_to_pay: String,
    pub merchant_id: String,
    pub amount: i64,
    pub currency: Option<storage_enums::Currency>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_modified_at: Option<PrimitiveDateTime>,
    // #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub fullfilment_time: Option<PrimitiveDateTime>,
}
