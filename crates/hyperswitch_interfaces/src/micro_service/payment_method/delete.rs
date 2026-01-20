use api_models::payment_methods::PaymentMethodId;
use common_utils::request::Method;
use serde::Deserialize;

use crate::micro_service::{MicroserviceClientError, MicroserviceClientErrorKind};

const DUMMY_PM_ID: &str = "pm_dummy";

pub struct DeletePaymentMethod {
    pub payment_method_id: PaymentMethodId,
}

impl DeletePaymentMethod {
    pub fn new(payment_method_id: PaymentMethodId) -> Self {
        Self { payment_method_id }
    }
}

#[derive(Clone, Debug)]
// TODO: replace dummy request types with real v1/v2 models.
pub struct DeletePaymentMethodV2Request {
    pub payment_method_id: PaymentMethodId,
}

#[derive(Clone, Debug, Deserialize)]
// TODO: replace dummy response types with real v1/v2 models.
pub struct DeletePaymentMethodV2Response {
    pub id: String,
}

#[derive(Clone, Debug)]
// TODO: replace dummy response types with real v1/v2 models.
pub struct DeletePaymentMethodResponse {
    pub payment_method_id: String,
    pub deleted: Option<bool>,
}

impl TryFrom<&DeletePaymentMethod> for DeletePaymentMethodV2Request {
    type Error = MicroserviceClientError;

    fn try_from(value: &DeletePaymentMethod) -> Result<Self, Self::Error> {
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
    fn validate_request(&self) -> Result<(), MicroserviceClientError> {
        if self.payment_method_id.payment_method_id.trim().is_empty() {
            return Err(MicroserviceClientError {
                operation: std::any::type_name::<DeletePaymentMethod>().to_string(),
                kind: MicroserviceClientErrorKind::InvalidRequest(
                    "Payment method ID cannot be empty".to_string(),
                ),
            });
        }
        Ok(())
    }

    fn build_path_params(
        &self,
        request: &DeletePaymentMethodV2Request,
    ) -> Vec<(&'static str, String)> {
        vec![("id", request.payment_method_id.payment_method_id.clone())]
    }
}

crate::impl_microservice_flow!(
    DeletePaymentMethod,
    method = Method::Delete,
    path = "/v2/payment-methods/{id}",
    v2_request = DeletePaymentMethodV2Request,
    v2_response = DeletePaymentMethodV2Response,
    v1_response = DeletePaymentMethodResponse,
    client = crate::micro_service::payment_method::PaymentMethodClient<'_>,
    path_params = DeletePaymentMethod::build_path_params,
    validate = DeletePaymentMethod::validate_request
);
