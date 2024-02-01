use api_models::customers;
pub use api_models::customers::{CustomerDeleteResponse, CustomerId, CustomerRequest};
use serde::Serialize;

use super::payments;
use crate::{core::errors::RouterResult, newtype, types::domain};

newtype!(
    pub CustomerResponse = customers::CustomerResponse,
    derives = (Debug, Clone, Serialize)
);

impl common_utils::events::ApiEventMetric for CustomerResponse {
        /// Returns the API event type if it exists, otherwise returns None.
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        self.0.get_api_event_type()
    }
}

pub(crate) trait CustomerRequestExt: Sized {
    fn validate(self) -> RouterResult<Self>;
}

impl From<(domain::Customer, Option<payments::AddressDetails>)> for CustomerResponse {
        /// Converts a tuple containing a Customer and an optional AddressDetails into a CustomerResponse
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
