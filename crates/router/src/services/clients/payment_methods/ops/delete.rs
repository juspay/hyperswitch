pub mod request;
pub mod response;
pub mod transformers;

use request::{DeleteV1Request, DeleteV2Request};
use response::{DeleteV1Response, DeleteV2Response};

use super::ClientOperation;
use crate::services::clients::payment_methods::error::PaymentMethodClientError;

pub struct DeletePaymentMethod {
    payment_method_id: String,
}

impl DeletePaymentMethod {
    pub fn new(payment_method_id: String) -> Self {
        Self { payment_method_id }
    }
}

#[async_trait::async_trait]
impl ClientOperation for DeletePaymentMethod {
    type V1Response = DeleteV1Response;
    type V2Request = DeleteV2Request;
    type V2Response = DeleteV2Response;

    fn operation(&self) -> &'static str {
        "delete_payment_method"
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
        let request = DeleteV1Request {
            payment_method_id: self.payment_method_id.clone(),
        };
        DeleteV2Request::try_from(&request)
    }

    async fn execute(
        &self,
        client: &crate::services::clients::payment_methods::ModularPaymentMethodClient<'_>,
        request: Self::V2Request,
    ) -> Result<Self::V2Response, PaymentMethodClientError> {
        let path = format!("/v2/payment-methods/{}", self.payment_method_id);
        let payload: DeleteV2Response = client
            .execute_request(
                common_utils::request::Method::Delete,
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
        DeleteV1Response::try_from(response)
    }
}
