//! Retrieve payment method flow types and models.

use api_models::payment_methods::{NetworkTokenResponse, PaymentMethodId};
use common_utils::{id_type, request::Method};
use hyperswitch_interfaces::micro_service::{MicroserviceClientError, MicroserviceClientErrorKind};
use time::PrimitiveDateTime;

use crate::types::{
    ConnectorTokenDetails, ModularPMRetrieveRequest, ModularPMRetrieveResponse,
    PaymentMethodResponseData, RawPaymentMethodData,
};
/// V1-facing retrieve flow type.
#[derive(Debug)]
pub struct RetrievePaymentMethod;

/// V1-facing retrieve request payload.
#[derive(Debug)]
pub struct RetrievePaymentMethodV1Request {
    pub payment_method_id: PaymentMethodId,
    pub modular_service_prefix: String,
    pub fetch_raw_detail: bool,
}
/// V1-facing retrieve response payload.
#[derive(Clone, Debug)]
pub struct RetrievePaymentMethodResponse {
    pub payment_method_id: String,
    pub merchant_id: id_type::MerchantId,
    pub customer_id: Option<id_type::CustomerId>,
    pub payment_method: common_enums::PaymentMethod,
    pub payment_method_type: common_enums::PaymentMethodType,
    pub recurring_enabled: Option<bool>,
    pub created: Option<PrimitiveDateTime>,
    pub last_used_at: Option<PrimitiveDateTime>,
    pub payment_method_data: Option<PaymentMethodResponseData>,
    pub connector_tokens: Option<Vec<ConnectorTokenDetails>>,
    pub network_token: Option<NetworkTokenResponse>,
    pub raw_payment_method_data: Option<RawPaymentMethodData>,
    pub network_transaction_id: Option<String>,
    pub billing: Option<hyperswitch_domain_models::address::Address>,
}

impl TryFrom<&RetrievePaymentMethodV1Request> for ModularPMRetrieveRequest {
    type Error = MicroserviceClientError;

    fn try_from(_value: &RetrievePaymentMethodV1Request) -> Result<Self, Self::Error> {
        Ok(Self)
    }
}

impl TryFrom<ModularPMRetrieveResponse> for RetrievePaymentMethodResponse {
    type Error = MicroserviceClientError;

    fn try_from(v2_resp: ModularPMRetrieveResponse) -> Result<Self, Self::Error> {
        // Extract payment_method_id from GlobalPaymentMethodId
        let payment_method_id = v2_resp.id.clone();

        // Convert GlobalCustomerId to CustomerId
        let customer_id = v2_resp.customer_id;

        Ok(Self {
            payment_method_id,
            merchant_id: v2_resp.merchant_id,
            customer_id,
            payment_method: v2_resp.payment_method_type,
            payment_method_type: v2_resp.payment_method_subtype,
            recurring_enabled: v2_resp.recurring_enabled,
            created: v2_resp.created,
            last_used_at: v2_resp.last_used_at,
            payment_method_data: v2_resp.payment_method_data,
            connector_tokens: v2_resp.connector_tokens,
            network_token: v2_resp.network_token,
            raw_payment_method_data: v2_resp.raw_payment_method_data,
            billing: v2_resp.billing,
            network_transaction_id: v2_resp.network_transaction_id,
        })
    }
}

impl RetrievePaymentMethod {
    fn validate_request(
        &self,
        request: &RetrievePaymentMethodV1Request,
    ) -> Result<(), MicroserviceClientError> {
        if request
            .payment_method_id
            .payment_method_id
            .trim()
            .is_empty()
        {
            return Err(MicroserviceClientError {
                operation: std::any::type_name::<Self>().to_string(),
                kind: MicroserviceClientErrorKind::InvalidRequest(
                    "Payment method ID cannot be empty".to_string(),
                ),
            });
        }
        Ok(())
    }

    fn build_path_params(
        &self,
        request: &RetrievePaymentMethodV1Request,
    ) -> Vec<(&'static str, String)> {
        vec![
            ("prefix", request.modular_service_prefix.clone()),
            ("id", request.payment_method_id.payment_method_id.clone()),
        ]
    }

    fn query_params(
        &self,
        request: &RetrievePaymentMethodV1Request,
    ) -> Vec<(&'static str, String)> {
        vec![("fetch_raw_detail", request.fetch_raw_detail.to_string())]
    }
}

hyperswitch_interfaces::impl_microservice_flow!(
    RetrievePaymentMethod,
    method = Method::Get,
    path = "/{prefix}/payment-methods/{id}",
    v1_request = RetrievePaymentMethodV1Request,
    v2_request = ModularPMRetrieveRequest,
    v2_response = ModularPMRetrieveResponse,
    v1_response = RetrievePaymentMethodResponse,
    client = crate::client::PaymentMethodClient<'_>,
    path_params = RetrievePaymentMethod::build_path_params,
    query_params = RetrievePaymentMethod::query_params,
    validate = RetrievePaymentMethod::validate_request
);
