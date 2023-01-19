use common_utils::{consts, custom_serde, pii};
use masking::Secret;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
    #[schema(max_length = 255, example = "Jon Test")]
    pub name: Option<String>,
    /// The customer's email address
    #[schema(value_type = Option<String>,max_length = 255, example = "JonTest@test.com")]
    pub email: Option<Secret<String, pii::Email>>,
    /// The customer's phone number
    #[schema(value_type = Option<String>,max_length = 255, example = "9999999999")]
    pub phone: Option<Secret<String>>,
    /// An arbitrary string that you can attach to a customer object.
    #[schema(max_length = 255, example = "First Customer")]
    pub description: Option<String>,
    /// The country code for the customer phone number
    #[schema(max_length = 255, example = "+65")]
    pub phone_country_code: Option<String>,
    /// The address for the customer
    #[schema(value_type = Option<Object>,example = json!({
    "city": "Bangalore",
    "country": "IN",
    "line1": "Juspay router",
    "line2": "Koramangala",
    "line3": "Stallion",
    "state": "Karnataka",
    "zip": "560095",
    "first_name": "John",
    "last_name": "Doe"
  }))]
    pub address: Option<Secret<serde_json::Value>>,
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500
    /// characters long. Metadata is useful for storing additional, structured information on an
    /// object.
    #[schema(value_type = Option<Object>,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CustomerResponse {
    /// The identifier for the customer object. If not provided the customer ID will be autogenerated.
    #[schema(max_length = 255, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub customer_id: String,
    /// The customer's name
    #[schema(max_length = 255, example = "Jon Test")]
    pub name: Option<String>,
    /// The customer's email address
    #[schema(value_type = Option<String>,max_length = 255, example = "JonTest@test.com")]
    pub email: Option<Secret<String, pii::Email>>,
    /// The customer's phone number
    #[schema(value_type = Option<String>,max_length = 255, example = "9999999999")]
    pub phone: Option<Secret<String>>,
    /// The country code for the customer phone number
    #[schema(max_length = 255, example = "+65")]
    pub phone_country_code: Option<String>,
    /// An arbitrary string that you can attach to a customer object.
    #[schema(max_length = 255, example = "First Customer")]
    pub description: Option<String>,
    /// The address for the customer
    #[schema(value_type = Option<Object>,example = json!({
    "city": "Bangalore",
    "country": "IN",
    "line1": "Juspay router",
    "line2": "Koramangala",
    "line3": "Stallion",
    "state": "Karnataka",
    "zip": "560095",
    "first_name": "John",
    "last_name": "Doe"
  }))]
    pub address: Option<Secret<serde_json::Value>>,
    ///  A timestamp (ISO 8601 code) that determines when the customer was created
    #[schema(value_type = PrimitiveDateTime,example = "2023-01-18T11:04:09.922Z")]
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: time::PrimitiveDateTime,
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500
    /// characters long. Metadata is useful for storing additional, structured information on an
    /// object.
    #[schema(value_type = Option<Object>,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Default, Debug, Deserialize, Serialize)]
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
