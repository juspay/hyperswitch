use common_utils::{consts, custom_serde, pii};
use masking::Secret;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct CustomerRequest {
    #[serde(default = "generate_customer_id")]
    pub customer_id: String,
    #[serde(default = "unknown_merchant", skip)]
    pub merchant_id: String,
    pub name: Option<String>,
    pub email: Option<Secret<String, pii::Email>>,
    pub phone: Option<Secret<String>>,
    pub description: Option<String>,
    pub phone_country_code: Option<String>,
    pub address: Option<Secret<serde_json::Value>>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CustomerResponse {
    pub customer_id: String,
    pub name: Option<String>,
    pub email: Option<Secret<String, pii::Email>>,
    pub phone: Option<Secret<String>>,
    pub phone_country_code: Option<String>,
    pub description: Option<String>,
    pub address: Option<Secret<serde_json::Value>>,
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: time::PrimitiveDateTime,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct CustomerId {
    pub customer_id: String,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct CustomerDeleteResponse {
    pub customer_id: String,
    pub customer_deleted: bool,
    pub address_deleted: bool,
    pub payment_methods_deleted: bool,
}

pub fn generate_customer_id() -> String {
    common_utils::generate_id(consts::ID_LENGTH, "cus")
}

fn unknown_merchant() -> String {
    String::from("merchant_unknown")
}
