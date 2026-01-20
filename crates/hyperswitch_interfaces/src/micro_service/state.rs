use common_utils::request::{Method, RequestContent};
use serde::de::DeserializeOwned;

use super::error::MicroserviceClientError;

/// Contract for a single microservice flow.
///
/// This trait defines the compile-time shape of a flow: how it validates inputs, how it derives a
/// modular service request payload, and how it maps the upstream response back into a v1 response.
/// The
/// executor orchestrates the pipeline and enforces the order:
///
/// Validated -> TransformedRequest -> Executed -> TransformedResponse
#[async_trait::async_trait]
pub trait ClientOperation {
    /// HTTP method for this flow, used by the executor when building the request.
    ///
    /// This is a compile-time constant so the executor can remain generic.
    const METHOD: Method;
    /// Path template for this flow (e.g., `/v2/payment-methods/{id}`).
    ///
    /// Use `{token}` placeholders and provide values via `path_params`.
    /// The executor performs string substitution for each token.
    const PATH_TEMPLATE: &'static str;

    /// V1-facing response type returned by the flow.
    ///
    /// This is the final output returned by `FlowType::call`, after all transforms.
    type V1Response;
    /// Modular service request payload produced by request transformation.
    ///
    /// This is not sent automatically; it is passed into `body()` and/or `path_params()`.
    type V2Request;
    /// Modular service response payload returned by the upstream service.
    ///
    /// The executor deserializes this from the upstream HTTP response body.
    type V2Response: DeserializeOwned;

    /// Validate inputs before building a request.
    ///
    /// Use this to reject invalid IDs or missing required fields. Failures are classified as
    /// client-side errors in the pipeline.
    fn validate(&self) -> Result<(), MicroserviceClientError>;
    /// Transform flow inputs into a modular service request payload.
    ///
    /// Should be a pure conversion without side effects. Do not perform I/O here.
    fn transform_request(&self) -> Result<Self::V2Request, MicroserviceClientError>;
    /// Transform modular service response payload into V1 response.
    ///
    /// Treat failures here as server-side transform errors. Keep it deterministic.
    fn transform_response(
        &self,
        response: Self::V2Response,
    ) -> Result<Self::V1Response, MicroserviceClientError>;

    /// Optional path parameters for template substitution.
    ///
    /// Keys should correspond to `{token}` placeholders in `PATH_TEMPLATE`.
    /// This is typically used for resource IDs.
    fn path_params(&self, _request: &Self::V2Request) -> Vec<(&'static str, String)> {
        Vec::new()
    }

    /// Optional body for the outbound request.
    ///
    /// Return `None` for methods that do not send a body.
    fn body(&self, _request: Self::V2Request) -> Option<RequestContent> {
        None
    }
}

/// State after validation succeeds.
#[derive(Debug)]
pub struct Validated<O>(pub(crate) O);

/// State after request transformation.
#[derive(Debug)]
pub struct TransformedRequest<O: ClientOperation> {
    /// Flow instance.
    pub(crate) op: O,
    /// Transformed request payload.
    pub(crate) request: O::V2Request,
}

/// State after executing the HTTP request.
#[derive(Debug)]
pub struct Executed<O: ClientOperation> {
    /// Flow instance.
    pub(crate) op: O,
    /// Upstream response payload.
    pub(crate) response: O::V2Response,
}

/// State after transforming into the V1 response.
#[derive(Debug)]
pub struct TransformedResponse<O: ClientOperation> {
    /// Final V1 response payload.
    pub output: O::V1Response,
    /// Marker to retain the flow type.
    pub(crate) _op: std::marker::PhantomData<O>,
}
