use api_models::customers;
pub use api_models::customers::{CustomerDeleteResponse, CustomerId, CustomerRequest};
use serde::Serialize;

use super::payments;
use crate::{core::errors::RouterResult, newtype, types::domain};

newtype!(
    pub CustomerResponse = customers::CustomerResponse,
    derives = (Debug, Clone, Serialize)
);

pub(crate) trait CustomerRequestExt: Sized {
    fn validate(self) -> RouterResult<Self>;
}

impl From<(domain::Customer, Option<payments::AddressDetails>)> for CustomerResponse {
    fn from((cust, address): (domain::Customer, Option<payments::AddressDetails>)) -> Self {
        customers::CustomerResponse {
            customer_id: cust.customer_id,
            name: cust.name,
            email: cust.email,
            phone: cust.phone,
            phone_country_code: cust.phone_country_code,
            description: cust.description,
            created_at: cust.created_at,
            metadata: cust.metadata,
            address,
        }
        .into()
    }
}
