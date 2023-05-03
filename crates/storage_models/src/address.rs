use common_utils::{consts, generate_id};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use time::PrimitiveDateTime;

use crate::{encryption::Encryption, enums, schema::address};

#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = address)]
// #[serde(deny_unknown_fields)]
pub struct AddressNew {
    pub address_id: String,
    pub city: Option<String>,
    pub country: Option<enums::CountryAlpha2>,
    pub line1: Option<Encryption>,
    pub line2: Option<Encryption>,
    pub line3: Option<Encryption>,
    pub state: Option<Encryption>,
    pub zip: Option<Encryption>,
    pub first_name: Option<Encryption>,
    pub last_name: Option<Encryption>,
    pub phone_number: Option<Encryption>,
    pub country_code: Option<String>,
    pub customer_id: String,
    pub merchant_id: String,
}

#[derive(Clone, Debug, Identifiable, Queryable, frunk::LabelledGeneric)]
#[diesel(table_name = address)]
pub struct Address {
    // #[serde(skip_serializing)]
    pub id: i32,
    // #[serde(skip_serializing)]
    pub address_id: String,
    pub city: Option<String>,
    pub country: Option<enums::CountryAlpha2>,
    pub line1: Option<Encryption>,
    pub line2: Option<Encryption>,
    pub line3: Option<Encryption>,
    pub state: Option<Encryption>,
    pub zip: Option<Encryption>,
    pub first_name: Option<Encryption>,
    pub last_name: Option<Encryption>,
    pub phone_number: Option<Encryption>,
    pub country_code: Option<String>,
    // #[serde(skip_serializing)]
    // #[serde(with = "custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    // #[serde(skip_serializing)]
    // #[serde(with = "custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    pub customer_id: String,
    pub merchant_id: String,
}

#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = address)]
pub struct AddressUpdateInternal {
    pub city: Option<String>,
    pub country: Option<enums::CountryAlpha2>,
    pub line1: Option<Encryption>,
    pub line2: Option<Encryption>,
    pub line3: Option<Encryption>,
    pub state: Option<Encryption>,
    pub zip: Option<Encryption>,
    pub first_name: Option<Encryption>,
    pub last_name: Option<Encryption>,
    pub phone_number: Option<Encryption>,
    pub country_code: Option<String>,
    pub modified_at: PrimitiveDateTime,
}

impl Default for AddressNew {
    fn default() -> Self {
        Self {
            address_id: generate_id(consts::ID_LENGTH, "add"),
            city: None,
            country: None,
            line1: None,
            line2: None,
            line3: None,
            state: None,
            zip: None,
            first_name: None,
            last_name: None,
            phone_number: None,
            country_code: None,
            customer_id: String::default(),
            merchant_id: String::default(),
        }
    }
}
