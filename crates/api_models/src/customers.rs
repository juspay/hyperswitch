use common_utils::{consts, crypto, custom_serde, pii};
use masking::Secret;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    enums as api_enums,
};

#[derive(Debug, Default, Clone, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AddressDetails {
    /// The address city
    #[schema(max_length = 50, example = "New York")]
    pub city: Option<String>,

    /// The two-letter ISO country code for the address
    #[schema(value_type = Option<CountryAlpha2>, example = "US")]
    pub country: Option<api_enums::CountryAlpha2>,

    /// The first line of the address
    #[schema(value_type = Option<String>, max_length = 200, example = "123, King Street")]
    pub line1: Option<Secret<String>>,

    /// The second line of the address
    #[schema(value_type = Option<String>, max_length = 50, example = "Powelson Avenue")]
    pub line2: Option<Secret<String>>,

    /// The third line of the address
    #[schema(value_type = Option<String>, max_length = 50, example = "Bridgewater")]
    pub line3: Option<Secret<String>>,

    /// The zip/postal code for the address
    #[schema(value_type = Option<String>, max_length = 50, example = "08807")]
    pub zip: Option<Secret<String>>,

    /// The address state
    #[schema(value_type = Option<String>, example = "New York")]
    pub state: Option<Secret<String>>,

    /// The first name for the address
    #[schema(value_type = Option<String>, max_length = 255, example = "John")]
    pub first_name: Option<Secret<String>>,

    /// The last name for the address
    #[schema(value_type = Option<String>, max_length = 255, example = "Doe")]
    pub last_name: Option<Secret<String>>,
}

/// The customer details
#[derive(Debug, Default, Clone, Deserialize, Serialize, ToSchema)]
pub struct CustomerRequest {
    /// The identifier for the customer object. If not provided the customer ID will be autogenerated.
    #[schema(max_length = 255, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    #[serde(default = "generate_customer_id")]
    pub customer_id: String,
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
    pub address: Option<AddressDetails>,
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500
    /// characters long. Metadata is useful for storing additional, structured information on an
    /// object.
    #[schema(value_type = Option<Object>,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<pii::SecretSerdeValue>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CustomerResponse {
    /// The identifier for the customer object. If not provided the customer ID will be autogenerated.
    #[schema(max_length = 255, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub customer_id: String,
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
    pub address: Option<AddressDetails>,
    ///  A timestamp (ISO 8601 code) that determines when the customer was created
    #[schema(value_type = PrimitiveDateTime,example = "2023-01-18T11:04:09.922Z")]
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: time::PrimitiveDateTime,
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500
    /// characters long. Metadata is useful for storing additional, structured information on an
    /// object.
    #[schema(value_type = Option<Object>,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<pii::SecretSerdeValue>,
}

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub struct CustomerId {
    pub customer_id: String,
}

#[derive(Default, Debug, Deserialize, Serialize, ToSchema)]
pub struct CustomerDeleteResponse {
    /// The identifier for the customer object
    #[schema(max_length = 255, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub customer_id: String,
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

pub fn generate_customer_id() -> String {
    common_utils::generate_id(consts::ID_LENGTH, "cus")
}

fn unknown_merchant() -> String {
    String::from("merchant_unknown")
}
