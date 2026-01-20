use api_models::payment_methods::PaymentMethodId;
use common_utils::request::Method;
use serde::Deserialize;

use crate::micro_service::{MicroserviceClientError, MicroserviceClientErrorKind};

const DUMMY_PM_ID: &str = "pm_dummy";

pub struct RetrievePaymentMethod {
    pub payment_method_id: PaymentMethodId,
}

impl RetrievePaymentMethod {
    pub fn new(payment_method_id: PaymentMethodId) -> Self {
        Self { payment_method_id }
    }
}

#[derive(Clone, Debug)]
// TODO: replace dummy request types with real v1/v2 models.
pub struct RetrievePaymentMethodV2Request {
    pub payment_method_id: PaymentMethodId,
}

#[derive(Clone, Debug, Deserialize)]
// TODO: replace dummy response types with real v1/v2 models.
pub struct RetrievePaymentMethodV2Response {
    pub id: String,
}

#[derive(Clone, Debug)]
// TODO: replace dummy response types with real v1/v2 models.
pub struct RetrievePaymentMethodResponse {
    pub payment_method_id: String,
    pub deleted: Option<bool>,
}

impl TryFrom<&RetrievePaymentMethod> for RetrievePaymentMethodV2Request {
    type Error = MicroserviceClientError;

    fn try_from(value: &RetrievePaymentMethod) -> Result<Self, Self::Error> {
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
    fn validate_request(&self) -> Result<(), MicroserviceClientError> {
        if self.payment_method_id.payment_method_id.trim().is_empty() {
            return Err(MicroserviceClientError {
                operation: std::any::type_name::<RetrievePaymentMethod>().to_string(),
                kind: MicroserviceClientErrorKind::InvalidRequest(
                    "Payment method ID cannot be empty".to_string(),
                ),
            });
        }
        Ok(())
    }

    fn build_path_params(
        &self,
        request: &RetrievePaymentMethodV2Request,
    ) -> Vec<(&'static str, String)> {
        vec![("id", request.payment_method_id.payment_method_id.clone())]
    }
}

crate::impl_microservice_flow!(
    RetrievePaymentMethod,
    method = Method::Get,
    path = "/v2/payment-methods/{id}",
    v2_request = RetrievePaymentMethodV2Request,
    v2_response = RetrievePaymentMethodV2Response,
    v1_response = RetrievePaymentMethodResponse,
    client = crate::micro_service::payment_method::PaymentMethodClient<'_>,
    path_params = RetrievePaymentMethod::build_path_params,
    validate = RetrievePaymentMethod::validate_request
);
