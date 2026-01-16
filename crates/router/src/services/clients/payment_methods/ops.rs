pub mod create;
pub mod delete;
pub mod retrieve;
pub mod update;

use crate::services::clients::payment_methods::{
    client::ModularPaymentMethodClient, error::PaymentMethodClientError,
};
use router_env::logger;

#[async_trait::async_trait]
pub trait ClientOperation {
    /// V1-facing response type returned by the client.
    type V1Response;
    /// V2 request payload produced after transformation.
    type V2Request;
    /// V2 response payload returned by the upstream service.
    type V2Response;

    /// Operation name used for logging and error attribution.
    fn operation(&self) -> &'static str;

    /// Validate request inputs; failures map to InvalidRequest.
    fn validate(&self) -> Result<(), PaymentMethodClientError>;

    /// Transform v1 inputs into a v2 request shape.
    fn transform_request(&self) -> Result<Self::V2Request, PaymentMethodClientError>;

    /// Execute the upstream HTTP call to the modular service.
    async fn execute(
        &self,
        client: &ModularPaymentMethodClient<'_>,
        request: Self::V2Request,
    ) -> Result<Self::V2Response, PaymentMethodClientError>;

    /// Transform the v2 response into a v1 response shape.
    fn transform_response(
        &self,
        response: Self::V2Response,
    ) -> Result<Self::V1Response, PaymentMethodClientError>;

    /// Run the full validate → transform → execute → transform pipeline.
    async fn run(
        self,
        client: &ModularPaymentMethodClient<'_>,
    ) -> Result<Self::V1Response, PaymentMethodClientError>
    where
        Self: Sized,
    {
        self.validate().map_err(|err| {
            logger::warn!(operation = self.operation(), error = ?err, "pm_client validation failed");
            err
        })?;
        let request = self.transform_request().map_err(|err| {
            logger::warn!(
                operation = self.operation(),
                error = ?err,
                "pm_client request transform failed"
            );
            err
        })?;
        let response = self.execute(client, request).await?;
        self.transform_response(response).map_err(|err| {
            logger::error!(
                operation = self.operation(),
                error = ?err,
                "pm_client response transform failed"
            );
            err
        })
    }
}
