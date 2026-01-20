//! Create payment method flow types and dummy models.

use common_utils::request::{Method, RequestContent};
use serde::Deserialize;
use serde_json::Value;

use crate::micro_service::MicroserviceClientError;

/// V1-facing create flow input.
#[derive(Debug)]
pub struct CreatePaymentMethod {
    /// Raw payload forwarded to the modular service.
    pub payload: Value,
}

const DUMMY_PM_ID: &str = "pm_dummy";
/// Dummy modular service request payload.
#[derive(Clone, Debug)]
// TODO: replace dummy request types with real v1/modular models.
pub struct CreatePaymentMethodV2Request {
    /// Payload to send in the request body.
    pub payload: Value,
}

/// Dummy modular service response payload.
#[derive(Clone, Debug, Deserialize)]
// TODO: replace dummy response types with real v1/modular models.
pub struct CreatePaymentMethodV2Response {
    /// Dummy identifier returned by the modular service.
    pub id: String,
}

/// V1-facing create response (dummy for now).
#[derive(Clone, Debug)]
// TODO: replace dummy response types with real v1/modular models.
pub struct CreatePaymentMethodResponse {
    /// V1 payment method identifier.
    pub payment_method_id: String,
    /// Dummy delete marker (unused).
    pub deleted: Option<bool>,
}

impl TryFrom<&CreatePaymentMethod> for CreatePaymentMethodV2Request {
    type Error = MicroserviceClientError;

    fn try_from(value: &CreatePaymentMethod) -> Result<Self, Self::Error> {
        Ok(Self {
            payload: value.payload.clone(),
        })
    }
}

impl TryFrom<CreatePaymentMethodV2Response> for CreatePaymentMethodResponse {
    type Error = MicroserviceClientError;

    fn try_from(_: CreatePaymentMethodV2Response) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_id: DUMMY_PM_ID.to_string(),
            deleted: None,
        })
    }
}

impl CreatePaymentMethod {
    fn build_body(&self, request: CreatePaymentMethodV2Request) -> Option<RequestContent> {
        Some(RequestContent::Json(Box::new(request.payload)))
    }
}

crate::impl_microservice_flow!(
    CreatePaymentMethod,
    method = Method::Post,
    path = "/v2/payment-methods",
    v2_request = CreatePaymentMethodV2Request,
    v2_response = CreatePaymentMethodV2Response,
    v1_response = CreatePaymentMethodResponse,
    client = crate::micro_service::payment_method::PaymentMethodClient<'_>,
    body = CreatePaymentMethod::build_body
);
