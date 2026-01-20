use common_utils::request::{Method, RequestContent};
use serde::Deserialize;
use serde_json::Value;

use crate::micro_service::MicroserviceClientError;

pub struct CreatePaymentMethod {
    pub payload: Value,
}

const DUMMY_PM_ID: &str = "pm_dummy";
#[derive(Clone, Debug)]
// TODO: replace dummy request types with real v1/v2 models.
pub struct CreatePaymentMethodV2Request {
    pub payload: Value,
}

#[derive(Clone, Debug, Deserialize)]
// TODO: replace dummy response types with real v1/v2 models.
pub struct CreatePaymentMethodV2Response {
    pub id: String,
}

#[derive(Clone, Debug)]
// TODO: replace dummy response types with real v1/v2 models.
pub struct CreatePaymentMethodResponse {
    pub payment_method_id: String,
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
