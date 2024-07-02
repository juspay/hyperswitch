use common_utils::{
    crypto, custom_serde,
    encryption::Encryption,
    id_type,
    pii::{self, EmailStrategy},
    types::keymanager::ToEncryptable,
};
use euclid::dssa::graph::euclid_graph_prelude::FxHashMap;
use masking::{ExposeInterface, Secret, SwitchStrategy};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::payments;

/// The customer details
#[derive(Debug, Default, Clone, Deserialize, Serialize, ToSchema)]
pub struct CustomerRequest {
    /// The identifier for the customer object. If not provided the customer ID will be autogenerated.
    #[schema(value_type = Option<String>, max_length = 64, min_length = 1, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub customer_id: Option<id_type::CustomerId>,
    /// The identifier for the Merchant Account
    #[schema(max_length = 255, example = "y3oqhf46pyzuxjbcn2giaqnb44")]
    #[serde(default = "unknown_merchant", skip)]
    pub merchant_id: String,
    /// The customer's name
    #[schema(max_length = 255, value_type = Option<String>, example = "Jon Test")]
    pub name: Option<Secret<String>>,
    /// The customer's email address
    #[schema(value_type = Option<String>, max_length = 255, example = "JonTest@test.com")]
    pub email: Option<pii::Email>,
    /// The customer's phone number
    #[schema(value_type = Option<String>, max_length = 255, example = "9999999999")]
    pub phone: Option<Secret<String>>,
    /// An arbitrary string that you can attach to a customer object.
    #[schema(max_length = 255, example = "First Customer")]
    pub description: Option<String>,
    /// The country code for the customer phone number
    #[schema(max_length = 255, example = "+65")]
    pub phone_country_code: Option<String>,
    /// The address for the customer
    #[schema(value_type = Option<AddressDetails>)]
    pub address: Option<payments::AddressDetails>,
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500
    /// characters long. Metadata is useful for storing additional, structured information on an
    /// object.
    #[schema(value_type = Option<Object>,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<pii::SecretSerdeValue>,
}

pub struct CustomerRequestWithEmail {
    pub name: Option<Secret<String>>,
    pub email: Option<pii::Email>,
    pub phone: Option<Secret<String>>,
}

pub struct CustomerRequestWithEncryption {
    pub name: Option<Encryption>,
    pub phone: Option<Encryption>,
    pub email: Option<Encryption>,
}

pub struct EncryptableCustomer {
    pub name: crypto::OptionalEncryptableName,
    pub phone: crypto::OptionalEncryptablePhone,
    pub email: crypto::OptionalEncryptableEmail,
}

impl ToEncryptable<EncryptableCustomer, Secret<String>, Encryption>
    for CustomerRequestWithEncryption
{
    fn to_encryptable(self) -> FxHashMap<String, Encryption> {
        let mut map = FxHashMap::with_capacity_and_hasher(3, Default::default());
        self.name.map(|x| map.insert("name".to_string(), x));
        self.phone.map(|x| map.insert("phone".to_string(), x));
        self.email.map(|x| map.insert("email".to_string(), x));
        map
    }

    fn from_encryptable(
        hashmap: FxHashMap<String, crypto::Encryptable<Secret<String>>>,
    ) -> common_utils::errors::CustomResult<EncryptableCustomer, common_utils::errors::ParsingError>
    {
        Ok(EncryptableCustomer {
            name: hashmap.get("name").cloned(),
            phone: hashmap.get("phone").cloned(),
            email: hashmap.get("email").cloned().map(|email| {
                let encryptable: crypto::Encryptable<Secret<String, EmailStrategy>> =
                    crypto::Encryptable::new(
                        email.clone().into_inner().switch_strategy(),
                        email.into_encrypted(),
                    );
                encryptable
            }),
        })
    }
}

impl ToEncryptable<EncryptableCustomer, Secret<String>, Secret<String>>
    for CustomerRequestWithEmail
{
    fn to_encryptable(self) -> FxHashMap<String, Secret<String>> {
        let mut map = FxHashMap::with_capacity_and_hasher(3, Default::default());
        self.name.map(|x| map.insert("name".to_string(), x));
        self.phone.map(|x| map.insert("phone".to_string(), x));
        self.email
            .map(|x| map.insert("email".to_string(), x.expose().switch_strategy()));
        map
    }

    fn from_encryptable(
        hashmap: FxHashMap<String, crypto::Encryptable<Secret<String>>>,
    ) -> common_utils::errors::CustomResult<EncryptableCustomer, common_utils::errors::ParsingError>
    {
        Ok(EncryptableCustomer {
            name: hashmap.get("name").cloned(),
            email: hashmap.get("email").cloned().map(|email| {
                let encryptable: crypto::Encryptable<Secret<String, EmailStrategy>> =
                    crypto::Encryptable::new(
                        email.clone().into_inner().switch_strategy(),
                        email.into_encrypted(),
                    );
                encryptable
            }),
            phone: hashmap.get("phone").cloned(),
        })
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CustomerResponse {
    /// The identifier for the customer object
    #[schema(value_type = String, max_length = 64, min_length = 1, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub customer_id: id_type::CustomerId,
    /// The customer's name
    #[schema(max_length = 255, value_type = Option<String>, example = "Jon Test")]
    pub name: crypto::OptionalEncryptableName,
    /// The customer's email address
    #[schema(value_type = Option<String>,max_length = 255, example = "JonTest@test.com")]
    pub email: crypto::OptionalEncryptableEmail,
    /// The customer's phone number
    #[schema(value_type = Option<String>,max_length = 255, example = "9999999999")]
    pub phone: crypto::OptionalEncryptablePhone,
    /// The country code for the customer phone number
    #[schema(max_length = 255, example = "+65")]
    pub phone_country_code: Option<String>,
    /// An arbitrary string that you can attach to a customer object.
    #[schema(max_length = 255, example = "First Customer")]
    pub description: Option<String>,
    /// The address for the customer
    #[schema(value_type = Option<AddressDetails>)]
    pub address: Option<payments::AddressDetails>,
    ///  A timestamp (ISO 8601 code) that determines when the customer was created
    #[schema(value_type = PrimitiveDateTime,example = "2023-01-18T11:04:09.922Z")]
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: time::PrimitiveDateTime,
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500
    /// characters long. Metadata is useful for storing additional, structured information on an
    /// object.
    #[schema(value_type = Option<Object>,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<pii::SecretSerdeValue>,
    /// The identifier for the default payment method.
    #[schema(max_length = 64, example = "pm_djh2837dwduh890123")]
    pub default_payment_method_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CustomerId {
    pub customer_id: id_type::CustomerId,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CustomerDeleteResponse {
    /// The identifier for the customer object
    #[schema(value_type = String, max_length = 255, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub customer_id: id_type::CustomerId,
    /// Whether customer was deleted or not
    #[schema(example = false)]
    pub customer_deleted: bool,
    /// Whether address was deleted or not
    #[schema(example = false)]
    pub address_deleted: bool,
    /// Whether payment methods deleted or not
    #[schema(example = false)]
    pub payment_methods_deleted: bool,
}

fn unknown_merchant() -> String {
    String::from("merchant_unknown")
}
