use common_utils::{consts, custom_serde, date_time, generate_id};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, PrimitiveDateTime};

use crate::{enums, schema::address};

#[derive(Clone, Debug, Deserialize, Serialize, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = address)]
#[serde(deny_unknown_fields)]
pub struct AddressNew {
    pub address_id: String,
    pub city: Option<String>,
    pub country: Option<enums::CountryAlpha2>,
    pub line1: Option<Secret<String>>,
    pub line2: Option<Secret<String>>,
    pub line3: Option<Secret<String>>,
    pub state: Option<Secret<String>>,
    pub zip: Option<Secret<String>>,
    pub first_name: Option<Secret<String>>,
    pub last_name: Option<Secret<String>>,
    pub phone_number: Option<Secret<String>>,
    pub country_code: Option<String>,
    pub customer_id: String,
    pub merchant_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, Identifiable, Queryable, frunk::LabelledGeneric)]
#[diesel(table_name = address)]
pub struct Address {
    #[serde(skip_serializing)]
    pub id: i32,
    #[serde(skip_serializing)]
    pub address_id: String,
    pub city: Option<String>,
    pub country: Option<enums::CountryAlpha2>,
    pub line1: Option<Secret<String>>,
    pub line2: Option<Secret<String>>,
    pub line3: Option<Secret<String>>,
    pub state: Option<Secret<String>>,
    pub zip: Option<Secret<String>>,
    pub first_name: Option<Secret<String>>,
    pub last_name: Option<Secret<String>>,
    pub phone_number: Option<Secret<String>>,
    pub country_code: Option<String>,
    #[serde(skip_serializing)]
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(skip_serializing)]
    #[serde(with = "custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    pub customer_id: String,
    pub merchant_id: String,
}

#[derive(Debug, frunk::LabelledGeneric)]
pub enum AddressUpdate {
    Update {
        city: Option<String>,
        country: Option<enums::CountryAlpha2>,
        line1: Option<Secret<String>>,
        line2: Option<Secret<String>>,
        line3: Option<Secret<String>>,
        state: Option<Secret<String>>,
        zip: Option<Secret<String>>,
        first_name: Option<Secret<String>>,
        last_name: Option<Secret<String>>,
        phone_number: Option<Secret<String>>,
        country_code: Option<String>,
    },
}

#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = address)]
pub struct AddressUpdateInternal {
    city: Option<String>,
    country: Option<enums::CountryAlpha2>,
    line1: Option<Secret<String>>,
    line2: Option<Secret<String>>,
    line3: Option<Secret<String>>,
    state: Option<Secret<String>>,
    zip: Option<Secret<String>>,
    first_name: Option<Secret<String>>,
    last_name: Option<Secret<String>>,
    phone_number: Option<Secret<String>>,
    country_code: Option<String>,
    modified_at: PrimitiveDateTime,
}

impl From<AddressUpdate> for AddressUpdateInternal {
    fn from(address_update: AddressUpdate) -> Self {
        match address_update {
            AddressUpdate::Update {
                city,
                country,
                line1,
                line2,
                line3,
                state,
                zip,
                first_name,
                last_name,
                phone_number,
                country_code,
            } => Self {
                city,
                country,
                line1,
                line2,
                line3,
                state,
                zip,
                first_name,
                last_name,
                phone_number,
                country_code,
                modified_at: date_time::convert_to_pdt(OffsetDateTime::now_utc()),
            },
        }
    }
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
