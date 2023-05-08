use api_models::customers;
pub use api_models::customers::{CustomerDeleteResponse, CustomerId, CustomerRequest};
use serde::Serialize;

use crate::{newtype, types::storage};

newtype!(
    pub CustomerResponse = customers::CustomerResponse,
    derives = (Debug, Clone, Serialize)
);

impl From<storage::Customer> for CustomerResponse {
    fn from(cust: storage::Customer) -> Self {
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
