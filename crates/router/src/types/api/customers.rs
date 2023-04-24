use api_models::customers;
pub use api_models::customers::{CustomerDeleteResponse, CustomerId, CustomerRequest};
use error_stack::ResultExt;
use serde::Serialize;

use crate::{
    core::errors::{self, RouterResult},
    newtype,
    pii::PeekInterface,
    types::domain::customer as domain,
    utils::{self, ValidateCall},
};

newtype!(
    pub CustomerResponse = customers::CustomerResponse,
    derives = (Debug, Clone, Serialize)
);

pub(crate) trait CustomerRequestExt: Sized {
    fn validate(self) -> RouterResult<Self>;
}

impl CustomerRequestExt for CustomerRequest {
    fn validate(self) -> RouterResult<Self> {
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

impl From<domain::Customer> for CustomerResponse {
    fn from(cust: domain::Customer) -> Self {
        customers::CustomerResponse {
            customer_id: cust.customer_id,
            name: cust.name,
            email: cust.email,
            phone: cust.phone,
            phone_country_code: cust.phone_country_code,
            description: cust.description,
            created_at: cust.created_at,
            metadata: cust.metadata,
            address: None,
        }
        .into()
    }
}
