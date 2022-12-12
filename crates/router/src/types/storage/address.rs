use common_utils::custom_serde;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, PrimitiveDateTime};

use crate::{consts, schema::address, types::api, utils::generate_id};

#[derive(Clone, Debug, Deserialize, Serialize, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = address)]
#[serde(deny_unknown_fields)]
pub struct AddressNew {
    pub address_id: String,
    pub city: Option<String>,
    pub country: Option<String>,
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

#[derive(Clone, Debug, Deserialize, Serialize, Identifiable, Queryable)]
#[diesel(table_name = address)]
pub struct Address {
    #[serde(skip_serializing)]
    pub id: i32,
    #[serde(skip_serializing)]
    pub address_id: String,
    pub city: Option<String>,
    pub country: Option<String>,
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

#[derive(Debug)]
pub enum AddressUpdate {
    Update {
        city: Option<String>,
        country: Option<String>,
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
pub(super) struct AddressUpdateInternal {
    city: Option<String>,
    country: Option<String>,
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
                modified_at: convert_to_pdt(OffsetDateTime::now_utc()),
            },
        }
    }
}

// TODO: Create utils for this since cane be reused outside address
fn convert_to_pdt(offset_time: OffsetDateTime) -> PrimitiveDateTime {
    PrimitiveDateTime::new(offset_time.date(), offset_time.time())
}

impl<'a> From<&'a api::Address> for AddressUpdate {
    fn from(address: &api::Address) -> Self {
        AddressUpdate::Update {
            city: address.address.as_ref().and_then(|a| a.city.clone()),
            country: address.address.as_ref().and_then(|a| a.country.clone()),
            line1: address.address.as_ref().and_then(|a| a.line1.clone()),
            line2: address.address.as_ref().and_then(|a| a.line2.clone()),
            line3: address.address.as_ref().and_then(|a| a.line3.clone()),
            state: address.address.as_ref().and_then(|a| a.state.clone()),
            zip: address.address.as_ref().and_then(|a| a.zip.clone()),
            first_name: address.address.as_ref().and_then(|a| a.first_name.clone()),
            last_name: address.address.as_ref().and_then(|a| a.last_name.clone()),
            phone_number: address.phone.as_ref().and_then(|a| a.number.clone()),
            country_code: address.phone.as_ref().and_then(|a| a.country_code.clone()),
        }
    }
}

impl<'a> From<&'a Address> for api::Address {
    fn from(address: &Address) -> Self {
        api::Address {
            address: Some(api::AddressDetails {
                city: address.city.clone(),
                country: address.country.clone(),
                line1: address.line1.clone(),
                line2: address.line2.clone(),
                line3: address.line3.clone(),
                state: address.state.clone(),
                zip: address.zip.clone(),
                first_name: address.first_name.clone(),
                last_name: address.last_name.clone(),
            }),
            phone: Some(api::PhoneDetails {
                number: address.phone_number.clone(),
                country_code: address.country_code.clone(),
            }),
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
