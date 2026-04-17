use api_models::customers;
pub use api_models::customers::{
    CustomerDeleteResponse, CustomerDocumentDetails, CustomerListRequest,
    CustomerListRequestWithConstraints, CustomerListResponse, CustomerRequest,
    CustomerUpdateRequest, CustomerUpdateRequestInternal,
};
use common_utils::ext_traits::ValueExt;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::customer;
use serde::Serialize;

#[cfg(feature = "v1")]
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

#[cfg(feature = "v1")]
impl TryFrom<(domain::Customer, Option<payments::AddressDetails>)> for CustomerResponse {
    type Error = error_stack::Report<common_utils::errors::ParsingError>;
    fn try_from(
        (cust, address): (domain::Customer, Option<payments::AddressDetails>),
    ) -> Result<Self, Self::Error> {
        let document_details = cust
            .document_details
            .as_ref()
            .map(|encryptable| {
                encryptable
                    .clone()
                    .into_inner()
                    .parse_value::<CustomerDocumentDetails>("CustomerDocumentDetails")
                    .map_err(|err| {
                        router_env::logger::error!(?err, "Failed to parse CustomerDocumentDetails");
                        err
                    })
            })
            .transpose()?;
        Ok(Self(customers::CustomerResponse {
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
            document_details,
        }))
    }
}

#[cfg(feature = "v2")]
impl TryFrom<customer::Customer> for CustomerResponse {
    type Error = error_stack::Report<common_utils::errors::ParsingError>;
    fn try_from(cust: customer::Customer) -> Result<Self, Self::Error> {
        let document_details = cust
            .document_details
            .as_ref()
            .map(|encryptable| {
                encryptable
                    .clone()
                    .into_inner()
                    .parse_value::<CustomerDocumentDetails>("CustomerDocumentDetails")
                    .map_err(|err| {
                        router_env::logger::error!(?err, "Failed to parse CustomerDocumentDetails");
                        err
                    })
            })
            .transpose()?;

        let default_billing_address = cust
            .default_billing_address
            .as_ref()
            .map(|encryptable| {
                encryptable
                    .clone()
                    .into_inner()
                    .parse_value::<api_models::payments::AddressDetails>("AddressDetails")
                    .map_err(|err| {
                        router_env::logger::error!(?err, "Failed to parse default_billing_address");
                        err
                    })
            })
            .transpose()?;

        let default_shipping_address = cust
            .default_shipping_address
            .as_ref()
            .map(|encryptable| {
                encryptable
                    .clone()
                    .into_inner()
                    .parse_value::<api_models::payments::AddressDetails>("AddressDetails")
                    .map_err(|err| {
                        router_env::logger::error!(
                            ?err,
                            "Failed to parse default_shipping_address"
                        );
                        err
                    })
            })
            .transpose()?;

        Ok(Self(customers::CustomerResponse {
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
            default_billing_address,
            default_shipping_address,
            default_payment_method_id: cust.default_payment_method_id,
            tax_registration_id: cust.tax_registration_id,
            document_details,
        }))
    }
}
