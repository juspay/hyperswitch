use common_utils::request::{Method, RequestContent};
use serde::de::DeserializeOwned;

use super::error::MicroserviceClientError;

#[async_trait::async_trait]
pub trait ClientOperation {
    const METHOD: Method;
    const PATH_TEMPLATE: &'static str;

    type V1Response;
    type V2Request;
    type V2Response: DeserializeOwned;

    fn validate(&self) -> Result<(), MicroserviceClientError>;
    fn transform_request(&self) -> Result<Self::V2Request, MicroserviceClientError>;
    fn transform_response(
        &self,
        response: Self::V2Response,
    ) -> Result<Self::V1Response, MicroserviceClientError>;

    fn path_params(&self, _request: &Self::V2Request) -> Vec<(&'static str, String)> {
        Vec::new()
    }

    fn body(&self, _request: Self::V2Request) -> Option<RequestContent> {
        None
    }
}

pub struct Validated<O>(pub(crate) O);

pub struct TransformedRequest<O: ClientOperation> {
    pub(crate) op: O,
    pub(crate) request: O::V2Request,
}

pub struct Executed<O: ClientOperation> {
    pub(crate) op: O,
    pub(crate) response: O::V2Response,
}

pub struct TransformedResponse<O: ClientOperation> {
    pub output: O::V1Response,
    pub(crate) _op: std::marker::PhantomData<O>,
}
