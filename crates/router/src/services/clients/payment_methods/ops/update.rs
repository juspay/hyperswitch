pub mod request;
pub mod response;
pub mod transformers;

use super::ClientOperation;
use crate::services::clients::payment_methods::error::PaymentMethodClientError;
use request::{UpdateV1Request, UpdateV2Request};
use response::{UpdateV1Response, UpdateV2Response};

pub struct UpdatePaymentMethod {
    payment_method_id: String,
    payload: serde_json::Value,
}

impl UpdatePaymentMethod {
    pub fn new(payment_method_id: String, payload: serde_json::Value) -> Self {
        Self {
            payment_method_id,
            payload,
        }
    }
}

#[async_trait::async_trait]
impl ClientOperation for UpdatePaymentMethod {
    type V1Response = UpdateV1Response;
    type V2Request = UpdateV2Request;
    type V2Response = UpdateV2Response;

    fn operation(&self) -> &'static str {
        "update_payment_method"
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
        let request = UpdateV1Request {
            payment_method_id: self.payment_method_id.clone(),
            payload: self.payload.clone(),
        };
        UpdateV2Request::try_from(&request)
    }

    async fn execute(
        &self,
        client: &crate::services::clients::payment_methods::ModularPaymentMethodClient<'_>,
        request: Self::V2Request,
    ) -> Result<Self::V2Response, PaymentMethodClientError> {
        let path = format!(
            "/v2/payment-methods/{}/update-saved-payment-method",
            self.payment_method_id
        );
        let payload: UpdateV2Response = client
            .execute_request(
            common_utils::request::Method::Patch,
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
        UpdateV1Response::try_from(response)
    }
}
