use common_utils::pii;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use time::PrimitiveDateTime;

use crate::{encryption::Encryption, schema::customers};

#[derive(
    Clone, Debug, Insertable, router_derive::DebugAsDisplay, serde::Deserialize, serde::Serialize,
)]
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

impl From<CustomerNew> for Customer {
    fn from(customer_new: CustomerNew) -> Self {
        Self {
            id: 0i32,
            customer_id: customer_new.customer_id,
            merchant_id: customer_new.merchant_id,
            name: customer_new.name,
            email: customer_new.email,
            phone: customer_new.phone,
            phone_country_code: customer_new.phone_country_code,
            description: customer_new.description,
            created_at: customer_new.created_at,
            metadata: customer_new.metadata,
            connector_customer: customer_new.connector_customer,
            modified_at: customer_new.modified_at,
            address_id: customer_new.address_id,
            default_payment_method_id: None,
        }
    }
}

#[derive(Clone, Debug, Identifiable, Queryable, serde::Deserialize, serde::Serialize)]
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

#[derive(
    Clone,
    Debug,
    Default,
    AsChangeset,
    router_derive::DebugAsDisplay,
    serde::Deserialize,
    serde::Serialize,
)]
#[diesel(table_name = customers)]
pub struct CustomerUpdateInternal {
    pub name: Option<Encryption>,
    pub email: Option<Encryption>,
    pub phone: Option<Encryption>,
    pub description: Option<String>,
    pub phone_country_code: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub modified_at: Option<PrimitiveDateTime>,
    pub connector_customer: Option<Option<serde_json::Value>>,
    pub address_id: Option<String>,
    pub default_payment_method_id: Option<Option<String>>,
}

impl CustomerUpdateInternal {
    pub fn apply_changeset(self, source: Customer) -> Customer {
        let Self {
            name,
            email,
            phone,
            description,
            phone_country_code,
            metadata,
            connector_customer,
            address_id,
            default_payment_method_id,
            ..
        } = self;

        Customer {
            name: name.map_or(source.name, Some),
            email: email.map_or(source.email, Some),
            phone: phone.map_or(source.phone, Some),
            description: description.map_or(source.description, Some),
            phone_country_code: phone_country_code.map_or(source.phone_country_code, Some),
            metadata: metadata.map_or(source.metadata, Some),
            modified_at: common_utils::date_time::now(),
            connector_customer: connector_customer
                .flatten()
                .map_or(source.connector_customer, Some),
            address_id: address_id.map_or(source.address_id, Some),
            default_payment_method_id: default_payment_method_id
                .flatten()
                .map_or(source.default_payment_method_id, Some),
            ..source
        }
    }
}
