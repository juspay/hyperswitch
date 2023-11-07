use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{encryption::Encryption, enums, schema::address};

#[derive(Clone, Debug, Insertable, Serialize, Deserialize, router_derive::DebugAsDisplay)]
#[diesel(table_name = address)]
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
    pub customer_id: Option<String>,
    pub merchant_id: String,
    pub payment_id: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub modified_at: PrimitiveDateTime,
    pub updated_by: String,
}

#[derive(Clone, Debug, Queryable, Identifiable, Serialize, Deserialize)]
#[diesel(table_name = address, primary_key(address_id))]
pub struct Address {
    pub id: Option<i32>,
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
    pub created_at: PrimitiveDateTime,
    pub modified_at: PrimitiveDateTime,
    pub customer_id: Option<String>,
    pub merchant_id: String,
    pub payment_id: Option<String>,
    pub updated_by: String,
}

#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay, Serialize, Deserialize)]
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
    pub updated_by: String,
}

impl AddressUpdateInternal {
    pub fn create_address(self, source: Address) -> Address {
        Address {
            city: self.city,
            country: self.country,
            line1: self.line1,
            line2: self.line2,
            line3: self.line3,
            state: self.state,
            zip: self.zip,
            first_name: self.first_name,
            last_name: self.last_name,
            phone_number: self.phone_number,
            country_code: self.country_code,
            modified_at: self.modified_at,
            updated_by: self.updated_by,
            ..source
        }
    }
}
