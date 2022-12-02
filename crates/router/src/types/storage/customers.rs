use common_utils::custom_serde;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use error_stack::ResultExt;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::{
    core::errors::{self, RouterResult},
    pii::{self, PeekInterface, Secret},
    schema::customers,
    utils::{self, ValidateCall},
};

#[derive(
    Default, Clone, Debug, Insertable, Deserialize, Serialize, router_derive::DebugAsDisplay,
)]
#[diesel(table_name = customers)]
#[serde(deny_unknown_fields)]
pub struct CustomerNew {
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

impl CustomerNew {
    // FIXME: Use ValidateVar trait?
    pub(crate) fn validate(self) -> RouterResult<Self> {
        self.email
            .as_ref()
            .validate_opt(|email| utils::validate_email(email.peek()))
            .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                field_name: "email".to_string(),
                expected_format: "valid email address".to_string(),
            })?;
        self.address
            .as_ref()
            .validate_opt(|addr| utils::validate_address(addr.peek()))
            .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                field_name: "address".to_string(),
                expected_format: "valid address".to_string(),
            })?;
        Ok(self)
    }
}

pub fn generate_customer_id() -> String {
    String::from("cus_") + &(Uuid::new_v4().to_string())
}

fn unknown_merchant() -> String {
    String::from("merchant_unknown")
}

#[derive(Clone, Debug, Identifiable, Queryable, Deserialize, Serialize)]
#[diesel(table_name = customers)]
pub struct Customer {
    #[serde(skip_serializing)]
    pub id: i32,
    pub customer_id: String,
    #[serde(skip_serializing)]
    pub merchant_id: String,
    pub name: Option<String>,
    pub email: Option<Secret<String, pii::Email>>,
    pub phone: Option<Secret<String>>,
    pub phone_country_code: Option<String>,
    pub description: Option<String>,
    pub address: Option<Secret<serde_json::Value>>,
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug)]
pub enum CustomerUpdate {
    Update {
        name: Option<String>,
        email: Option<Secret<String, pii::Email>>,
        phone: Option<Secret<String>>,
        description: Option<String>,
        phone_country_code: Option<String>,
        address: Option<Secret<serde_json::Value>>,
        metadata: Option<serde_json::Value>,
    },
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = customers)]
pub(super) struct CustomerUpdateInternal {
    name: Option<String>,
    email: Option<Secret<String, pii::Email>>,
    phone: Option<Secret<String>>,
    description: Option<String>,
    phone_country_code: Option<String>,
    address: Option<Secret<serde_json::Value>>,
    metadata: Option<serde_json::Value>,
}

impl From<CustomerUpdate> for CustomerUpdateInternal {
    fn from(customer_update: CustomerUpdate) -> Self {
        match customer_update {
            CustomerUpdate::Update {
                name,
                email,
                phone,
                description,
                phone_country_code,
                address,
                metadata,
            } => Self {
                name,
                email,
                phone,
                description,
                phone_country_code,
                address,
                metadata,
            },
        }
    }
}
