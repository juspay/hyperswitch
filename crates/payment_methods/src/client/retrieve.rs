//! Retrieve payment method flow types and models.

use api_models::payment_methods::{
    PaymentMethodId, PaymentMethodResponse as RetrievePaymentMethodResponse,
};
use common_utils::request::Method;
use hyperswitch_interfaces::micro_service::{MicroserviceClientError, MicroserviceClientErrorKind};

use crate::types::{
    ModularPMRetrieveResponse, ModularPMRetrieveResquest, PaymentMethodResponseData,
};
/// V1-facing retrieve flow type.
#[derive(Debug)]
pub struct RetrievePaymentMethod;

/// V1-facing retrieve request payload.
#[derive(Debug)]
pub struct RetrievePaymentMethodV1Request {
    pub payment_method_id: PaymentMethodId,
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
        // Extract payment_method_id from GlobalPaymentMethodId
        let payment_method_id = v2_resp.id.clone();

        // Convert GlobalCustomerId to CustomerId
        let customer_id = v2_resp.customer_id;

        // Convert card details from V2 to V1 format
        let card = v2_resp.payment_method_data.map(|data| match data {
            PaymentMethodResponseData::Card(card_detail) => card_detail,
        });

        Ok(Self {
            payment_method_id,
            merchant_id: v2_resp.merchant_id,
            customer_id,
            payment_method: v2_resp.payment_method_type,
            payment_method_type: v2_resp.payment_method_subtype,
            card,
            recurring_enabled: v2_resp.recurring_enabled,
            created: v2_resp.created,
            last_used_at: v2_resp.last_used_at,
            installment_payment_enabled: None,
            payment_experience: None,
            metadata: None,
            bank_transfer: None,
            client_secret: None,
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
