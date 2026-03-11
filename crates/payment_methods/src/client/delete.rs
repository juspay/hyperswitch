//! Delete payment method flow types and dummy models.

use api_models::payment_methods::PaymentMethodId;
use common_utils::request::Method;
use hyperswitch_interfaces::micro_service::{MicroserviceClientError, MicroserviceClientErrorKind};
use serde::Deserialize;

const DUMMY_PM_ID: &str = "pm_dummy";

/// V1-facing delete flow type.
#[derive(Debug)]
pub struct DeletePaymentMethod;

/// V1-facing delete request payload.
#[derive(Debug)]
pub struct DeletePaymentMethodV1Request {
    /// Identifier for the payment method to delete.
    pub payment_method_id: PaymentMethodId,
}

/// Dummy modular service request payload.
#[derive(Clone, Debug)]
// TODO: replace dummy request types with real v1/modular models.
pub struct DeletePaymentMethodV2Request {
    /// Identifier for the payment method to delete.
    pub payment_method_id: PaymentMethodId,
}

/// Dummy modular service response payload.
#[derive(Clone, Debug, Deserialize)]
// TODO: replace dummy response types with real v1/modular models.
pub struct DeletePaymentMethodV2Response {
    /// Dummy identifier returned by the modular service.
    pub id: String,
}

/// V1-facing delete response (dummy for now).
#[derive(Clone, Debug)]
// TODO: replace dummy response types with real v1/modular models.
pub struct DeletePaymentMethodResponse {
    /// V1 payment method identifier.
    pub payment_method_id: String,
    /// Dummy delete marker.
    pub deleted: Option<bool>,
}

impl TryFrom<&DeletePaymentMethodV1Request> for DeletePaymentMethodV2Request {
    type Error = MicroserviceClientError;

    fn try_from(value: &DeletePaymentMethodV1Request) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_id: value.payment_method_id.clone(),
        })
    }
}

impl TryFrom<DeletePaymentMethodV2Response> for DeletePaymentMethodResponse {
    type Error = MicroserviceClientError;

    fn try_from(_: DeletePaymentMethodV2Response) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_id: DUMMY_PM_ID.to_string(),
            deleted: Some(true),
        })
    }
}

impl DeletePaymentMethod {
    fn validate_request(
        &self,
        request: &DeletePaymentMethodV1Request,
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
        request: &DeletePaymentMethodV1Request,
    ) -> Vec<(&'static str, String)> {
        vec![("id", request.payment_method_id.payment_method_id.clone())]
    }
}

hyperswitch_interfaces::impl_microservice_flow!(
    DeletePaymentMethod,
    method = Method::Delete,
    path = "/v2/payment-methods/{id}",
    v1_request = DeletePaymentMethodV1Request,
    v2_request = DeletePaymentMethodV2Request,
    v2_response = DeletePaymentMethodV2Response,
    v1_response = DeletePaymentMethodResponse,
    client = crate::client::PaymentMethodClient<'_>,
    path_params = DeletePaymentMethod::build_path_params,
    validate = DeletePaymentMethod::validate_request
);
