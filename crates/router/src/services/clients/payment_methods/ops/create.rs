pub mod request;
pub mod response;
pub mod transformers;

use request::{CreateV1Request, CreateV2Request};
use response::{CreateV1Response, CreateV2Response};

use super::ClientOperation;
use crate::services::clients::payment_methods::error::PaymentMethodClientError;

pub struct CreatePaymentMethod {
    payload: serde_json::Value,
}

impl CreatePaymentMethod {
    pub fn new(payload: serde_json::Value) -> Self {
        Self { payload }
    }
}

#[async_trait::async_trait]
impl ClientOperation for CreatePaymentMethod {
    type V1Response = CreateV1Response;
    type V2Request = CreateV2Request;
    type V2Response = CreateV2Response;

    fn operation(&self) -> &'static str {
        "create_payment_method"
    }

    fn validate(&self) -> Result<(), PaymentMethodClientError> {
        Ok(())
    }

    fn transform_request(&self) -> Result<Self::V2Request, PaymentMethodClientError> {
        let request = CreateV1Request {
            payload: self.payload.clone(),
        };
        CreateV2Request::try_from(&request)
    }

    async fn execute(
        &self,
        client: &crate::services::clients::payment_methods::ModularPaymentMethodClient<'_>,
        request: Self::V2Request,
    ) -> Result<Self::V2Response, PaymentMethodClientError> {
        let path = "/v2/payment-methods";
        let payload: CreateV2Response = client
            .execute_request(
                common_utils::request::Method::Post,
                path,
                request.body,
                self.operation(),
            )
            .await?;
        Ok(payload)
    }

    fn transform_response(
        &self,
        response: Self::V2Response,
    ) -> Result<Self::V1Response, PaymentMethodClientError> {
        CreateV1Response::try_from(response)
    }
}
