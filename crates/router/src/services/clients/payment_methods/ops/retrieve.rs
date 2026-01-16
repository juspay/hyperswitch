pub mod request;
pub mod response;
pub mod transformers;

use super::ClientOperation;
use crate::services::clients::payment_methods::error::PaymentMethodClientError;
use request::{RetrieveV1Request, RetrieveV2Request};
use response::{RetrieveV1Response, RetrieveV2Response};

pub struct RetrievePaymentMethod {
    payment_method_id: String,
}

impl RetrievePaymentMethod {
    pub fn new(payment_method_id: String) -> Self {
        Self { payment_method_id }
    }
}

#[async_trait::async_trait]
impl ClientOperation for RetrievePaymentMethod {
    type V1Response = RetrieveV1Response;
    type V2Request = RetrieveV2Request;
    type V2Response = RetrieveV2Response;

    fn operation(&self) -> &'static str {
        "retrieve_payment_method"
    }

    fn validate(&self) -> Result<(), PaymentMethodClientError> {
        if self.payment_method_id.trim().is_empty() {
            return Err(PaymentMethodClientError::InvalidRequest {
                operation: self.operation().to_string(),
                message: "Payment method ID cannot be empty".to_string(),
            });
        }
        Ok(())
    }

    fn transform_request(&self) -> Result<Self::V2Request, PaymentMethodClientError> {
        let request = RetrieveV1Request {
            payment_method_id: self.payment_method_id.clone(),
        };
        RetrieveV2Request::try_from(&request)
    }

    async fn execute(
        &self,
        client: &crate::services::clients::payment_methods::ModularPaymentMethodClient<'_>,
        request: Self::V2Request,
    ) -> Result<Self::V2Response, PaymentMethodClientError> {
        let path = format!("/v2/payment-methods/{}", self.payment_method_id);
        let payload: RetrieveV2Response = client
            .execute_request(
            common_utils::request::Method::Get,
            &path,
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
        RetrieveV1Response::try_from(response)
    }
}
