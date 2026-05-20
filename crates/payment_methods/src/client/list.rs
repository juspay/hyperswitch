use api_models::payment_methods::PaymentMethodListRequest;
use common_utils::{id_type, request::Method};
use hyperswitch_interfaces::micro_service::{MicroserviceClientError, MicroserviceClientErrorKind};

use crate::types::{
    ModularListCustomerPaymentMethodsRequest, ModularListCustomerPaymentMethodsResponse,
};

/// V1-facing retrieve flow type.
#[derive(Debug)]
pub struct ListCustomerPaymentMethods;

/// V1-facing retrieve request payload.
#[derive(Debug)]
pub struct ListCustomerPaymentMethodsV1Request {
    pub customer_id: id_type::CustomerId,
    pub query_params: PaymentMethodListRequest,
    pub modular_service_prefix: String,
}

impl TryFrom<&ListCustomerPaymentMethodsV1Request> for ModularListCustomerPaymentMethodsRequest {
    type Error = MicroserviceClientError;

    fn try_from(_value: &ListCustomerPaymentMethodsV1Request) -> Result<Self, Self::Error> {
        Ok(Self)
    }
}

impl ListCustomerPaymentMethods {
    fn validate_request(
        &self,
        request: &ListCustomerPaymentMethodsV1Request,
    ) -> Result<(), MicroserviceClientError> {
        if request.customer_id.get_string_repr().trim().is_empty() {
            return Err(MicroserviceClientError {
                operation: std::any::type_name::<Self>().to_string(),
                kind: MicroserviceClientErrorKind::InvalidRequest(
                    "Customer ID cannot be empty".to_string(),
                ),
            });
        }
        Ok(())
    }

    fn build_path_params(
        &self,
        request: &ListCustomerPaymentMethodsV1Request,
    ) -> Vec<(&'static str, String)> {
        vec![
            ("prefix", request.modular_service_prefix.clone()),
            (
                "customer_id",
                request.customer_id.get_string_repr().to_string(),
            ),
        ]
    }

    fn query_params(
        &self,
        request: &ListCustomerPaymentMethodsV1Request,
    ) -> Vec<(&'static str, String)> {
        let mut params = Vec::new();

        let qp = &request.query_params;

        if let Some(secret) = &qp.client_secret {
            params.push(("client_secret", secret.clone()));
        }

        if let Some(amount) = qp.amount {
            params.push(("amount", amount.to_string()));
        }

        if let Some(recurring) = qp.recurring_enabled {
            params.push(("recurring_enabled", recurring.to_string()));
        }

        if let Some(limit) = qp.limit {
            params.push(("limit", limit.to_string()));
        }

        if let Some(countries) = &qp.accepted_countries {
            if let Ok(serialized) = serde_json::to_string(countries) {
                params.push(("accepted_countries", serialized));
            }
        }

        if let Some(currencies) = &qp.accepted_currencies {
            if let Ok(serialized) = serde_json::to_string(currencies) {
                params.push(("accepted_currencies", serialized));
            }
        }

        if let Some(networks) = &qp.card_networks {
            if let Ok(serialized) = serde_json::to_string(networks) {
                params.push(("card_networks", serialized));
            }
        }

        params
    }
}

pub struct ListCustomerPaymentMethodsRawResponse(pub ModularListCustomerPaymentMethodsResponse);

impl TryFrom<ModularListCustomerPaymentMethodsResponse> for ListCustomerPaymentMethodsRawResponse {
    type Error = MicroserviceClientError;
    fn try_from(value: ModularListCustomerPaymentMethodsResponse) -> Result<Self, Self::Error> {
        Ok(Self(value))
    }
}

hyperswitch_interfaces::impl_microservice_flow!(
    ListCustomerPaymentMethods,
    method = Method::Get,
    path = "/{prefix}/customers/{customer_id}/saved-payment-methods",
    v1_request = ListCustomerPaymentMethodsV1Request,
    v2_request = ModularListCustomerPaymentMethodsRequest,
    v2_response = ModularListCustomerPaymentMethodsResponse,
    v1_response = ListCustomerPaymentMethodsRawResponse,
    client = crate::client::PaymentMethodClient<'_>,
    path_params = ListCustomerPaymentMethods::build_path_params,
    query_params = ListCustomerPaymentMethods::query_params,
    validate = ListCustomerPaymentMethods::validate_request
);
