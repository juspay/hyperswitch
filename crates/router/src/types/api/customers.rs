use api_models::customers;
pub use api_models::customers::{CustomerDeleteResponse, CustomerId, CustomerRequest};
use serde::Serialize;

use super::payments;
use crate::{newtype, types::domain};

newtype!(
    pub CustomerResponse = customers::CustomerResponse,
    derives = (Debug, Clone, Serialize)
);

impl common_utils::events::ApiEventMetric for CustomerResponse {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        self.0.get_api_event_type()
    }
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
            default_payment_method_id: cust.default_payment_method_id,
        }
        .into()
    }
}
