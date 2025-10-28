use api_models::customers;
pub use api_models::customers::{
    CustomerDeleteResponse, CustomerListRequest, CustomerListRequestWithConstraints,
    CustomerListResponse, CustomerRequest, CustomerUpdateRequest, CustomerUpdateRequestInternal,
};
#[cfg(feature = "v2")]
use hyperswitch_domain_models::customer;
use serde::Serialize;

#[cfg(feature = "v1")]
use super::payments;
use crate::{
    newtype,
    types::{domain, ForeignFrom},
};

newtype!(
    pub CustomerResponse = customers::CustomerResponse,
    derives = (Debug, Clone, Serialize)
);

impl common_utils::events::ApiEventMetric for CustomerResponse {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        self.0.get_api_event_type()
    }
}

#[cfg(feature = "v1")]
impl ForeignFrom<(domain::Customer, Option<payments::AddressDetails>)> for CustomerResponse {
    fn foreign_from((cust, address): (domain::Customer, Option<payments::AddressDetails>)) -> Self {
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
            tax_registration_id: cust.tax_registration_id,
        }
        .into()
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<customer::Customer> for CustomerResponse {
    fn foreign_from(cust: domain::Customer) -> Self {
        customers::CustomerResponse {
            id: cust.id,
            merchant_reference_id: cust.merchant_reference_id,
            connector_customer_ids: cust.connector_customer,
            name: cust.name,
            email: cust.email,
            phone: cust.phone,
            phone_country_code: cust.phone_country_code,
            description: cust.description,
            created_at: cust.created_at,
            metadata: cust.metadata,
            default_billing_address: None,
            default_shipping_address: None,
            default_payment_method_id: cust.default_payment_method_id,
            tax_registration_id: cust.tax_registration_id,
        }
        .into()
    }
}
