use common_utils::pii;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use time::PrimitiveDateTime;

use crate::{encryption::Encryption, schema::customers};

#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = customers)]
pub struct CustomerNew {
    pub customer_id: String,
    pub merchant_id: String,
    pub name: Option<Encryption>,
    pub email: Option<Encryption>,
    pub phone: Option<Encryption>,
    pub description: Option<String>,
    pub phone_country_code: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_customer: Option<serde_json::Value>,
    pub created_at: PrimitiveDateTime,
    pub modified_at: PrimitiveDateTime,
    pub address_id: Option<String>,
}

#[derive(Clone, Debug, Identifiable, Queryable)]
#[diesel(table_name = customers)]
pub struct Customer {
    pub id: i32,
    pub customer_id: String,
    pub merchant_id: String,
    pub name: Option<Encryption>,
    pub email: Option<Encryption>,
    pub phone: Option<Encryption>,
    pub phone_country_code: Option<String>,
    pub description: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_customer: Option<serde_json::Value>,
    pub modified_at: PrimitiveDateTime,
    pub address_id: Option<String>,
    pub default_payment_method_id: Option<String>,
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = customers)]
pub struct CustomerUpdateInternal {
    pub name: Option<Encryption>,
    pub email: Option<Encryption>,
    pub phone: Option<Encryption>,
    pub description: Option<String>,
    pub phone_country_code: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub modified_at: Option<PrimitiveDateTime>,
    pub connector_customer: Option<serde_json::Value>,
    pub address_id: Option<String>,
    pub default_payment_method_id: Option<Option<String>>,
}
