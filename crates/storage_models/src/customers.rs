use common_utils::{pii, pii::Email};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use masking::Secret;
use time::PrimitiveDateTime;

use crate::schema::customers;

#[derive(Default, Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = customers)]
pub struct CustomerNew {
    pub customer_id: String,
    pub merchant_id: String,
    pub name: Option<String>,
    pub email: Option<Email>,
    pub phone: Option<Secret<String>>,
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
    pub name: Option<String>,
    pub email: Option<Email>,
    pub phone: Option<Secret<String>>,
    pub phone_country_code: Option<String>,
    pub description: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub modified_at: PrimitiveDateTime,
}

#[derive(Debug)]
pub enum CustomerUpdate {
    Update {
        name: Option<String>,
        email: Option<Email>,
        phone: Option<Secret<String>>,
        description: Option<String>,
        phone_country_code: Option<String>,
        metadata: Option<pii::SecretSerdeValue>,
    },
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = customers)]
pub struct CustomerUpdateInternal {
    name: Option<String>,
    email: Option<Email>,
    phone: Option<Secret<String>>,
    description: Option<String>,
    phone_country_code: Option<String>,
    metadata: Option<pii::SecretSerdeValue>,
    modified_at: Option<PrimitiveDateTime>,
}

impl From<CustomerUpdate> for CustomerUpdateInternal {
    fn from(customer_update: CustomerUpdate) -> Self {
        match customer_update {
            CustomerUpdate::Update {
                name,
                email,
                phone,
                description,
                phone_country_code,
                metadata,
            } => Self {
                name,
                email,
                phone,
                description,
                phone_country_code,
                metadata,
                modified_at: Some(common_utils::date_time::now()),
            },
        }
    }
}
