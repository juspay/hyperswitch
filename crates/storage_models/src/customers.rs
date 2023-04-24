use common_utils::pii;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use time::PrimitiveDateTime;

use crate::{encryption::Encryption, schema::customers};

#[derive(Default, Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
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
    pub modified_at: PrimitiveDateTime,
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
}
