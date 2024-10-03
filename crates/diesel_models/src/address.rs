use common_utils::{
    crypto::{self, Encryptable},
    encryption::Encryption,
    pii::EmailStrategy,
    types::keymanager::ToEncryptable,
};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use masking::{Secret, SwitchStrategy};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{enums, schema::address};

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
    pub customer_id: Option<common_utils::id_type::CustomerId>,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub payment_id: Option<common_utils::id_type::PaymentId>,
    pub created_at: PrimitiveDateTime,
    pub modified_at: PrimitiveDateTime,
    pub updated_by: String,
    pub email: Option<Encryption>,
}

#[derive(Clone, Debug, Queryable, Identifiable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = address, primary_key(address_id), check_for_backend(diesel::pg::Pg))]
pub struct Address {
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
    pub customer_id: Option<common_utils::id_type::CustomerId>,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub payment_id: Option<common_utils::id_type::PaymentId>,
    pub updated_by: String,
    pub email: Option<Encryption>,
}

#[derive(Clone)]
// Intermediate struct to convert HashMap to Address
pub struct EncryptableAddress {
    pub line1: crypto::OptionalEncryptableSecretString,
    pub line2: crypto::OptionalEncryptableSecretString,
    pub line3: crypto::OptionalEncryptableSecretString,
    pub state: crypto::OptionalEncryptableSecretString,
    pub zip: crypto::OptionalEncryptableSecretString,
    pub first_name: crypto::OptionalEncryptableSecretString,
    pub last_name: crypto::OptionalEncryptableSecretString,
    pub phone_number: crypto::OptionalEncryptableSecretString,
    pub email: crypto::OptionalEncryptableEmail,
}

impl ToEncryptable<EncryptableAddress, Secret<String>, Encryption> for Address {
    fn to_encryptable(self) -> FxHashMap<String, Encryption> {
        let mut map = FxHashMap::with_capacity_and_hasher(9, Default::default());
        self.line1.map(|x| map.insert("line1".to_string(), x));
        self.line2.map(|x| map.insert("line2".to_string(), x));
        self.line3.map(|x| map.insert("line3".to_string(), x));
        self.zip.map(|x| map.insert("zip".to_string(), x));
        self.state.map(|x| map.insert("state".to_string(), x));
        self.first_name
            .map(|x| map.insert("first_name".to_string(), x));
        self.last_name
            .map(|x| map.insert("last_name".to_string(), x));
        self.phone_number
            .map(|x| map.insert("phone_number".to_string(), x));
        self.email.map(|x| map.insert("email".to_string(), x));
        map
    }

    fn from_encryptable(
        mut hashmap: FxHashMap<String, Encryptable<Secret<String>>>,
    ) -> common_utils::errors::CustomResult<EncryptableAddress, common_utils::errors::ParsingError>
    {
        Ok(EncryptableAddress {
            line1: hashmap.remove("line1"),
            line2: hashmap.remove("line2"),
            line3: hashmap.remove("line3"),
            zip: hashmap.remove("zip"),
            state: hashmap.remove("state"),
            first_name: hashmap.remove("first_name"),
            last_name: hashmap.remove("last_name"),
            phone_number: hashmap.remove("phone_number"),
            email: hashmap.remove("email").map(|email| {
                let encryptable: Encryptable<Secret<String, EmailStrategy>> = Encryptable::new(
                    email.clone().into_inner().switch_strategy(),
                    email.into_encrypted(),
                );
                encryptable
            }),
        })
    }
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
    pub email: Option<Encryption>,
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
