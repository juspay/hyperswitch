use common_utils::{
    consts::TENANT_HEADER,
    request::{Headers, Request},
};
use masking::Maskable;
use router_env::{logger, RequestIdentifier};
use url::Url;

use super::{
    error::{MicroserviceClientError, MicroserviceClientErrorKind},
    state::{ClientOperation, Executed, TransformedRequest, TransformedResponse, Validated},
    MicroserviceClientContext,
};
use crate::api_client::{call_connector_api, ApiClientWrapper};

impl<O: ClientOperation> Validated<O> {
    /// Validate the flow and move into the `Validated` state.
    pub fn new(op: O, request: O::V1Request) -> Result<Self, MicroserviceClientError> {
        let operation = std::any::type_name::<O>();
        op.validate(&request).map_err(|err| {
            logger::warn!(operation, error = ?err, "microservice validation failed");
            err
        })?;
        Ok(Self { op, request })
    }

    /// Transform the validated flow into a request payload.
    pub fn into_transformed_request(
        self,
    ) -> Result<TransformedRequest<O>, MicroserviceClientError> {
        let operation = std::any::type_name::<O>();
        let request = self.op.transform_request(&self.request).map_err(|err| {
            logger::warn!(
                operation,
                error = ?err,
                "microservice request transform failed"
            );
            err
        })?;
        Ok(TransformedRequest {
            op: self.op,
            request,
        })
    }
}

impl<O: ClientOperation> TransformedRequest<O> {
    /// Execute the HTTP call for this operation and capture the raw response payload.
    pub async fn execute(
        self,
        state: &dyn ApiClientWrapper,
        base_url: &Url,
        parent_headers: Headers,
        trace_header: &RequestIdentifier,
    ) -> Result<Executed<O>, MicroserviceClientError> {
        let operation = std::any::type_name::<O>();
        // Step 1: Build path and URL.
        let path = {
            let mut path = O::PATH_TEMPLATE.to_string();
            for (key, value) in self.op.path_params(&self.request) {
                let token = format!("{{{key}}}");
                path = path.replace(&token, &value);
            }
            path
        };
        let url = base_url.join(&path).map_err(|e| {
            logger::error!(operation, error = ?e, "microservice URL join failed");
            MicroserviceClientError {
                operation: operation.to_string(),
                kind: MicroserviceClientErrorKind::Transport(format!(
                    "Failed to construct URL: {e}"
                )),
            }
        })?;

        // Step 2: Build headers and inject trace/request/tenant context.
        let mut http_request = Request::new(O::METHOD, url.as_str());
        http_request.headers = parent_headers;
        {
            let header_name = trace_header.header_name();
            let existing_id = state.get_request_id();
            let (trace_id, generated) = trace_header
                .id_reuse_strategy()
                .get_or_create_request_id(existing_id.as_ref());
            if generated {
                logger::debug!(
                    operation,
                    generated_id = %trace_id,
                    "trace header generated new request id"
                );
            }

            http_request.headers.insert((
                header_name.to_string(),
                Maskable::Normal(trace_id.to_string()),
            ));

            let tenant_id = state.get_tenant().tenant_id.get_string_repr().to_string();
            if !tenant_id.is_empty() {
                http_request
                    .headers
                    .insert((TENANT_HEADER.to_string(), Maskable::Normal(tenant_id)));
            }
        }

        // Step 3: Attach body (if any).
        if let Some(body) = self.op.body(self.request) {
            http_request.set_body(body);
        }

        // Step 4: Execute request and decode response.
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
                    kind: MicroserviceClientErrorKind::Deserialize(format!(
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
    /// Transform the upstream response into the v1 response shape.
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

/// Execute the full pipeline: validate → transform → execute → transform.
pub async fn execute_microservice_operation<O: ClientOperation>(
    state: &dyn ApiClientWrapper,
    client: &impl MicroserviceClientContext,
    request: O::V1Request,
) -> Result<O::V1Response, MicroserviceClientError> {
    let op = O::from_request(&request);
    let validated = Validated::new(op, request)?;
    let transformed = validated.into_transformed_request()?;
    let executed = transformed
        .execute(
            state,
            client.base_url(),
            client.parent_headers().clone(),
            client.trace(),
        )
        .await?;
    Ok(executed.into_transformed_response()?.output)
}
