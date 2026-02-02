use api_models::payment_methods::{CustomerPaymentMethod, PaymentMethodListRequest};
use common_utils::{id_type, request::Method};
use hyperswitch_interfaces::micro_service::{MicroserviceClientError, MicroserviceClientErrorKind};

use crate::types::{
    ModularListCustomerPaymentMethodsRequest, ModularListCustomerPaymentMethodsResponse,
    PaymentMethodResponseData,
};

/// V1-facing retrieve flow type.
#[derive(Debug)]
pub struct ListCustomerPaymentMethods;

/// V1-facing retrieve request payload.
#[derive(Debug)]
pub struct ListCustomerPaymentMethodsV1Request {
    pub customer_id: id_type::CustomerId,
    pub query_params: Option<PaymentMethodListRequest>,
    pub modular_service_prefix: String,
}
pub struct ListCustomerPaymentMethodsV1Response {
    pub customer_payment_methods: Vec<CustomerPaymentMethod>,
    pub is_guest_customer: Option<bool>,
}

impl TryFrom<&ListCustomerPaymentMethodsV1Request> for ModularListCustomerPaymentMethodsRequest {
    type Error = MicroserviceClientError;

    fn try_from(_value: &ListCustomerPaymentMethodsV1Request) -> Result<Self, Self::Error> {
        Ok(Self)
    }
}

impl TryFrom<ModularListCustomerPaymentMethodsResponse> for ListCustomerPaymentMethodsV1Response {
    type Error = MicroserviceClientError;

    fn try_from(
        v2_response: ModularListCustomerPaymentMethodsResponse,
    ) -> Result<Self, Self::Error> {
        let customer_payment_methods = v2_response
            .customer_payment_methods
            .into_iter()
            .map(|pm| CustomerPaymentMethod {
                payment_token: pm.id.clone(),
                payment_method_id: pm.id,
                customer_id: pm.customer_id,
                payment_method: pm.payment_method_type,
                payment_method_type: Some(pm.payment_method_subtype),
                payment_method_issuer: None,
                payment_method_issuer_code: None,
                recurring_enabled: pm.recurring_enabled,
                installment_payment_enabled: None,
                payment_experience: None,
                card: pm.payment_method_data.map(|data| match data {
                    PaymentMethodResponseData::Card(card_detail) => card_detail,
                }),
                metadata: None,
                created: Some(pm.created),
                bank_transfer: None,
                bank: pm.bank,
                surcharge_details: None,
                requires_cvv: pm.requires_cvv,
                last_used_at: Some(pm.last_used_at),
                default_payment_method_set: pm.is_default,
                billing: pm.billing,
            })
            .collect();

        Ok(Self {
            customer_payment_methods,
            is_guest_customer: None,
        })
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

        if let Some(qp) = qp {
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
        }
        params
    }
}

hyperswitch_interfaces::impl_microservice_flow!(
    ListCustomerPaymentMethods,
    method = Method::Get,
    path = "/{prefix}/customers/{customer_id}/saved-payment-methods",
    v1_request = ListCustomerPaymentMethodsV1Request,
    v2_request = ModularListCustomerPaymentMethodsRequest,
    v2_response = ModularListCustomerPaymentMethodsResponse,
    v1_response = ListCustomerPaymentMethodsV1Response,
    client = crate::client::PaymentMethodClient<'_>,
    path_params = ListCustomerPaymentMethods::build_path_params,
    query_params = ListCustomerPaymentMethods::query_params,
    validate = ListCustomerPaymentMethods::validate_request
);
