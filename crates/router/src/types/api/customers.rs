use common_utils::custom_serde;
use error_stack::ResultExt;
use serde::{Deserialize, Serialize};

use crate::{
    consts,
    core::errors::{self, RouterResult},
    pii::{self, PeekInterface, Secret},
    types::storage,
    utils::{self, ValidateCall},
};

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
    pub metadata: Option<serde_json::Value>,
}

impl CustomerRequest {
    pub(crate) fn validate(self) -> RouterResult<Self> {
        self.email
            .as_ref()
            .validate_opt(|email| utils::validate_email(email.peek()))
            .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                field_name: "email".to_string(),
                expected_format: "valid email address".to_string(),
            })?;

        Ok(self)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CustomerResponse {
    pub customer_id: String,
    pub name: Option<String>,
    pub email: Option<Secret<String, pii::Email>>,
    pub phone: Option<Secret<String>>,
    pub phone_country_code: Option<String>,
    pub description: Option<String>,
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: time::PrimitiveDateTime,
    pub metadata: Option<serde_json::Value>,
}

impl From<storage::Customer> for CustomerResponse {
    fn from(cust: storage::Customer) -> Self {
        Self {
            customer_id: cust.customer_id,
            name: cust.name,
            email: cust.email,
            phone: cust.phone,
            phone_country_code: cust.phone_country_code,
            description: cust.description,
            created_at: cust.created_at,
            metadata: cust.metadata,
        }
    }
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct CustomerId {
    pub customer_id: String,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct CustomerDeleteResponse {
    pub customer_id: String,
    pub deleted: bool,
}

pub fn generate_customer_id() -> String {
    utils::generate_id(consts::ID_LENGTH, "cus")
}

fn unknown_merchant() -> String {
    String::from("merchant_unknown")
}
