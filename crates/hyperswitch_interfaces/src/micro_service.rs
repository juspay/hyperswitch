use common_utils::{
    consts::{TENANT_HEADER, X_REQUEST_ID},
    request::{Headers, Method, Request, RequestContent},
};
use masking::Maskable;
use router_env::{logger, IdReuse, RequestIdentifier};
use serde::de::DeserializeOwned;
use thiserror::Error;
use url::Url;

use crate::api_client::{call_connector_api, ApiClientWrapper};

pub mod payment_method;

#[derive(Debug, Error)]
pub enum MicroserviceClientErrorKind {
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    #[error("Transport error: {0}")]
    Transport(String),
    #[error("Upstream error: status={status}, body={body}")]
    Upstream { status: u16, body: String },
    #[error("Serde error: {0}")]
    Serde(String),
    #[error("Response transform error: {0}")]
    ResponseTransform(String),
    #[error("Client specific error: {0}")]
    ClientSpecific(String),
}

#[derive(Debug, Error)]
#[error("Microservice client error for {operation}: {kind}")]
pub struct MicroserviceClientError {
    pub operation: String,
    pub kind: MicroserviceClientErrorKind,
}

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

pub struct Validated<O>(O);

pub struct TransformedRequest<O: ClientOperation> {
    op: O,
    request: O::V2Request,
}

pub struct Executed<O: ClientOperation> {
    op: O,
    response: O::V2Response,
}

pub struct TransformedResponse<O: ClientOperation> {
    pub output: O::V1Response,
    _op: std::marker::PhantomData<O>,
}

impl<O: ClientOperation> Validated<O> {
    pub fn new(op: O) -> Result<Self, MicroserviceClientError> {
        let operation = std::any::type_name::<O>();
        op.validate().map_err(|err| {
            logger::warn!(operation, error = ?err, "microservice validation failed");
            err
        })?;
        Ok(Self(op))
    }

    pub fn into_transformed_request(
        self,
    ) -> Result<TransformedRequest<O>, MicroserviceClientError> {
        let operation = std::any::type_name::<O>();
        let request = self.0.transform_request().map_err(|err| {
            logger::warn!(
                operation,
                error = ?err,
                "microservice request transform failed"
            );
            err
        })?;
        Ok(TransformedRequest {
            op: self.0,
            request,
        })
    }
}

impl<O: ClientOperation> TransformedRequest<O> {
    pub async fn execute(
        self,
        state: &dyn ApiClientWrapper,
        base_url: &Url,
        parent_headers: Headers,
        trace_header: &RequestIdentifier,
    ) -> Result<Executed<O>, MicroserviceClientError> {
        let operation = std::any::type_name::<O>();
        let path = build_path(O::PATH_TEMPLATE, self.op.path_params(&self.request));
        let url = base_url.join(&path).map_err(|e| {
            logger::error!(operation, error = ?e, "microservice URL join failed");
            MicroserviceClientError {
                operation: operation.to_string(),
                kind: MicroserviceClientErrorKind::Transport(format!(
                    "Failed to construct URL: {e}"
                )),
            }
        })?;

        let mut http_request = Request::new(O::METHOD, url.as_str());
        http_request.headers = parent_headers;
        inject_trace_headers(state, &mut http_request.headers, trace_header);

        if let Some(body) = self.op.body(self.request) {
            http_request.set_body(body);
        }

        let response = call_connector_api(state, http_request, operation)
            .await
            .map_err(|e| {
                logger::error!(operation, error = ?e, "microservice request failed");
                MicroserviceClientError {
                    operation: operation.to_string(),
                    kind: MicroserviceClientErrorKind::Transport(format!(
                        "Connector API error: {e}"
                    )),
                }
            })?;

        match response {
            Ok(success) => serde_json::from_slice(&success.response).map_err(|e| {
                logger::error!(
                    operation,
                    error = ?e,
                    "microservice response decode failed"
                );
                MicroserviceClientError {
                    operation: operation.to_string(),
                    kind: MicroserviceClientErrorKind::Serde(format!(
                        "Failed to parse response: {e}"
                    )),
                }
            }),
            Err(err_resp) => {
                logger::warn!(
                    operation,
                    status = err_resp.status_code,
                    "microservice upstream error"
                );
                let body = String::from_utf8_lossy(&err_resp.response);
                Err(MicroserviceClientError {
                    operation: operation.to_string(),
                    kind: MicroserviceClientErrorKind::Upstream {
                        status: err_resp.status_code,
                        body: body.chars().take(500).collect(),
                    },
                })
            }
        }
        .map(|response| Executed {
            op: self.op,
            response,
        })
    }
}

impl<O: ClientOperation> Executed<O> {
    pub fn into_transformed_response(
        self,
    ) -> Result<TransformedResponse<O>, MicroserviceClientError> {
        let operation = std::any::type_name::<O>();
        let output = self.op.transform_response(self.response).map_err(|err| {
            logger::error!(
                operation,
                error = ?err,
                "microservice response transform failed"
            );
            err
        })?;
        Ok(TransformedResponse {
            output,
            _op: std::marker::PhantomData,
        })
    }
}

pub async fn execute_microservice_operation<O: ClientOperation>(
    state: &dyn ApiClientWrapper,
    client: &payment_method::PaymentMethodClient<'_>,
    op: O,
) -> Result<O::V1Response, MicroserviceClientError> {
    let validated = Validated::new(op)?;
    let transformed = validated.into_transformed_request()?;
    let executed = transformed
        .execute(
            state,
            client.base_url.as_ref(),
            client.parent_headers.clone(),
            client.trace,
        )
        .await?;
    Ok(executed.into_transformed_response()?.output)
}

#[macro_export]
macro_rules! impl_microservice_flow {
    (
        $flow:ty,
        method = $method:expr,
        path = $path:expr,
        v2_request = $v2_req:ty,
        v2_response = $v2_resp:ty,
        v1_response = $v1_resp:ty,
        client = $client_ty:ty
        $(, body = $body_fn:expr)?
        $(, path_params = $path_params_fn:expr)?
        $(, validate = $validate_fn:expr)?
    ) => {
        #[async_trait::async_trait]
        impl $crate::micro_service::ClientOperation for $flow {
            const METHOD: common_utils::request::Method = $method;
            const PATH_TEMPLATE: &'static str = $path;

            type V1Response = $v1_resp;
            type V2Request = $v2_req;
            type V2Response = $v2_resp;

            fn validate(&self) -> Result<(), $crate::micro_service::MicroserviceClientError> {
                $($validate_fn(self)?;)?
                Ok(())
            }

            fn transform_request(
                &self,
            ) -> Result<Self::V2Request, $crate::micro_service::MicroserviceClientError> {
                <Self::V2Request as TryFrom<&Self>>::try_from(self)
            }

            fn transform_response(
                &self,
                response: Self::V2Response,
            ) -> Result<Self::V1Response, $crate::micro_service::MicroserviceClientError> {
                <Self::V1Response as TryFrom<Self::V2Response>>::try_from(response)
            }

            $(
            fn body(
                &self,
                request: Self::V2Request,
            ) -> Option<common_utils::request::RequestContent> {
                $body_fn(self, request)
            }
            )?

            $(
            fn path_params(
                &self,
                request: &Self::V2Request,
            ) -> Vec<(&'static str, String)> {
                $path_params_fn(self, request)
            }
            )?
        }

        impl $flow {
            pub async fn call(
                state: &dyn $crate::api_client::ApiClientWrapper,
                client: &$client_ty,
                request: Self,
            ) -> Result<
                <Self as $crate::micro_service::ClientOperation>::V1Response,
                $crate::micro_service::MicroserviceClientError,
            > {
                $crate::micro_service::execute_microservice_operation(
                    state,
                    client,
                    request,
                )
                .await
            }
        }
    };
}

fn build_path(template: &str, params: Vec<(&'static str, String)>) -> String {
    let mut path = template.to_string();
    for (key, value) in params {
        let token = format!("{{{key}}}");
        path = path.replace(&token, &value);
    }
    path
}

fn find_header(headers: &Headers, name: &str) -> Option<String> {
    headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.clone().into_inner())
}

fn inject_trace_headers(
    state: &dyn ApiClientWrapper,
    headers: &mut Headers,
    trace_header: &RequestIdentifier,
) {
    let header_name = trace_header.header_name();
    let trace_id = match trace_header.id_reuse_strategy() {
        IdReuse::UseIncoming => find_header(headers, header_name)
            .unwrap_or_else(common_utils::generate_time_ordered_id_without_prefix),
        IdReuse::IgnoreIncoming => common_utils::generate_time_ordered_id_without_prefix(),
    };

    headers.insert((header_name.to_string(), Maskable::Normal(trace_id)));

    if header_name != X_REQUEST_ID {
        if let Some(request_id) = state.get_request_id_str() {
            headers.insert((X_REQUEST_ID.to_string(), Maskable::Normal(request_id)));
        }
    }

    let tenant_id = state.get_tenant().tenant_id.get_string_repr().to_string();
    if !tenant_id.is_empty() {
        headers.insert((TENANT_HEADER.to_string(), Maskable::Normal(tenant_id)));
    }
}
