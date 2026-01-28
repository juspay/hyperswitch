//! Retrieve payment method flow types and dummy models.

use api_models::payment_methods::PaymentMethodId;
use common_utils::request::Method;
use hyperswitch_interfaces::micro_service::{MicroserviceClientError, MicroserviceClientErrorKind};
use serde::Deserialize;

const DUMMY_PM_ID: &str = "pm_dummy";

/// V1-facing retrieve flow type.
#[derive(Debug)]
pub struct RetrievePaymentMethod;

/// V1-facing retrieve request payload.
#[derive(Debug)]
pub struct RetrievePaymentMethodV1Request {
    /// Identifier for the payment method to fetch.
    pub payment_method_id: PaymentMethodId,
}

/// Dummy modular service request payload.
#[derive(Clone, Debug)]
// TODO: replace dummy request types with real v1/modular models.
pub struct RetrievePaymentMethodV2Request {
    /// Identifier for the payment method to fetch.
    pub payment_method_id: PaymentMethodId,
}

/// Dummy modular service response payload.
#[derive(Clone, Debug, Deserialize)]
// TODO: replace dummy response types with real v1/modular models.
pub struct RetrievePaymentMethodV2Response {
    /// Dummy identifier returned by the modular service.
    pub id: String,
}

/// V1-facing retrieve response (dummy for now).
#[derive(Clone, Debug)]
// TODO: replace dummy response types with real v1/modular models.
pub struct RetrievePaymentMethodResponse {
    /// V1 payment method identifier.
    pub payment_method_id: String,
    /// Dummy delete marker (unused).
    pub deleted: Option<bool>,
}

impl TryFrom<&RetrievePaymentMethodV1Request> for RetrievePaymentMethodV2Request {
    type Error = MicroserviceClientError;

    fn try_from(value: &RetrievePaymentMethodV1Request) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_id: value.payment_method_id.clone(),
        })
    }
}

impl TryFrom<RetrievePaymentMethodV2Response> for RetrievePaymentMethodResponse {
    type Error = MicroserviceClientError;

    fn try_from(_: RetrievePaymentMethodV2Response) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_id: DUMMY_PM_ID.to_string(),
            deleted: None,
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
    v2_request = RetrievePaymentMethodV2Request,
    v2_response = RetrievePaymentMethodV2Response,
    v1_response = RetrievePaymentMethodResponse,
    client = crate::client::PaymentMethodClient<'_>,
    path_params = RetrievePaymentMethod::build_path_params,
    validate = RetrievePaymentMethod::validate_request
);
