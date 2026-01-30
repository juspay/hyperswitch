//! Update payment method flow types and dummy models.

use api_models::payment_methods::PaymentMethodId;
use common_utils::request::{Method, RequestContent};
use hyperswitch_interfaces::micro_service::{MicroserviceClientError, MicroserviceClientErrorKind};
use serde::Deserialize;
use serde_json::Value;

const DUMMY_PM_ID: &str = "pm_dummy";

/// V1-facing update flow type.
#[derive(Debug)]
pub struct UpdatePaymentMethod;

/// V1-facing update request payload.
#[derive(Debug)]
pub struct UpdatePaymentMethodV1Request {
    /// Identifier for the payment method to update.
    pub payment_method_id: PaymentMethodId,
    /// Raw payload forwarded to the modular service.
    pub payload: Value,
}

/// Dummy modular service request payload.
#[derive(Clone, Debug)]
// TODO: replace dummy request types with real v1/modular models.
pub struct UpdatePaymentMethodV2Request {
    /// Identifier for the payment method to update.
    pub payment_method_id: PaymentMethodId,
    /// Payload to send in the request body.
    pub payload: Value,
}

/// Dummy modular service response payload.
#[derive(Clone, Debug, Deserialize)]
// TODO: replace dummy response types with real v1/modular models.
pub struct UpdatePaymentMethodV2Response {
    /// Dummy identifier returned by the modular service.
    pub id: String,
}

/// V1-facing update response (dummy for now).
#[derive(Clone, Debug)]
// TODO: replace dummy response types with real v1/modular models.
pub struct UpdatePaymentMethodResponse {
    /// V1 payment method identifier.
    pub payment_method_id: String,
    /// Dummy delete marker (unused).
    pub deleted: Option<bool>,
}

impl TryFrom<&UpdatePaymentMethodV1Request> for UpdatePaymentMethodV2Request {
    type Error = MicroserviceClientError;

    fn try_from(value: &UpdatePaymentMethodV1Request) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_id: value.payment_method_id.clone(),
            payload: value.payload.clone(),
        })
    }
}

impl TryFrom<UpdatePaymentMethodV2Response> for UpdatePaymentMethodResponse {
    type Error = MicroserviceClientError;

    fn try_from(_: UpdatePaymentMethodV2Response) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_id: DUMMY_PM_ID.to_string(),
            deleted: None,
        })
    }
}

impl UpdatePaymentMethod {
    fn validate_request(
        &self,
        request: &UpdatePaymentMethodV1Request,
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
        request: &UpdatePaymentMethodV1Request,
    ) -> Vec<(&'static str, String)> {
        vec![("id", request.payment_method_id.payment_method_id.clone())]
    }

    fn build_body(&self, request: UpdatePaymentMethodV2Request) -> Option<RequestContent> {
        Some(RequestContent::Json(Box::new(request.payload)))
    }
}

hyperswitch_interfaces::impl_microservice_flow!(
    UpdatePaymentMethod,
    method = Method::Patch,
    path = "/v2/payment-methods/{id}/update-saved-payment-method",
    v1_request = UpdatePaymentMethodV1Request,
    v2_request = UpdatePaymentMethodV2Request,
    v2_response = UpdatePaymentMethodV2Response,
    v1_response = UpdatePaymentMethodResponse,
    client = crate::client::PaymentMethodClient<'_>,
    body = UpdatePaymentMethod::build_body,
    path_params = UpdatePaymentMethod::build_path_params,
    validate = UpdatePaymentMethod::validate_request
);
