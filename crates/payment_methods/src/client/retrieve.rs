//! Retrieve payment method flow types and models.

use api_models::payment_methods::PaymentMethodId;
use common_utils::{id_type, request::Method};
use hyperswitch_interfaces::micro_service::{MicroserviceClientError, MicroserviceClientErrorKind};
use time::PrimitiveDateTime;

use crate::types::{
    ConnectorTokenDetails, ModularPMRetrieveResponse, ModularPMRetrieveResquest,
    NetworkTokenResponse, PaymentMethodResponseData, RawPaymentMethodData,
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
}

impl TryFrom<&RetrievePaymentMethodV1Request> for ModularPMRetrieveResquest {
    type Error = MicroserviceClientError;

    fn try_from(_value: &RetrievePaymentMethodV1Request) -> Result<Self, Self::Error> {
        Ok(Self)
    }
}

impl TryFrom<ModularPMRetrieveResponse> for RetrievePaymentMethodResponse {
    type Error = MicroserviceClientError;

    fn try_from(v2_resp: ModularPMRetrieveResponse) -> Result<Self, Self::Error> {
        let ModularPMRetrieveResponse {
            id,
            merchant_id,
            customer_id,
            payment_method_type,
            payment_method_subtype,
            recurring_enabled,
            created,
            last_used_at,
            payment_method_data,
            connector_tokens,
            network_token,
            storage_type: _,
            card_cvc_token_storage: _,
            raw_payment_method_data,
        } = v2_resp;

        let payment_method = payment_method_type.ok_or(MicroserviceClientError {
            operation: std::any::type_name::<Self>().to_string(),
            kind: MicroserviceClientErrorKind::ResponseTransform(
                "missing payment_method_type in retrieve response".to_string(),
            ),
        })?;

        let payment_method_type = payment_method_subtype.ok_or(MicroserviceClientError {
            operation: std::any::type_name::<Self>().to_string(),
            kind: MicroserviceClientErrorKind::ResponseTransform(
                "missing payment_method_subtype in retrieve response".to_string(),
            ),
        })?;

        Ok(Self {
            payment_method_id: id,
            merchant_id,
            customer_id,
            payment_method,
            payment_method_type,
            recurring_enabled,
            created,
            last_used_at,
            payment_method_data,
            connector_tokens,
            network_token,
            raw_payment_method_data,
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
        vec![("id", request.payment_method_id.payment_method_id.clone())]
    }
}

hyperswitch_interfaces::impl_microservice_flow!(
    RetrievePaymentMethod,
    method = Method::Get,
    path = "/v2/payment-methods/{id}",
    v1_request = RetrievePaymentMethodV1Request,
    v2_request = ModularPMRetrieveResquest,
    v2_response = ModularPMRetrieveResponse,
    v1_response = RetrievePaymentMethodResponse,
    client = crate::client::PaymentMethodClient<'_>,
    path_params = RetrievePaymentMethod::build_path_params,
    validate = RetrievePaymentMethod::validate_request
);
