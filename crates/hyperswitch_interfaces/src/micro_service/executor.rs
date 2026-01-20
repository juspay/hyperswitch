use common_utils::{
    consts::{TENANT_HEADER, X_REQUEST_ID},
    request::{Headers, Request, RequestContent},
};
use masking::Maskable;
use router_env::{logger, IdReuse, RequestIdentifier};
use url::Url;

use crate::api_client::{call_connector_api, ApiClientWrapper};

use super::error::{MicroserviceClientError, MicroserviceClientErrorKind};
use super::state::{ClientOperation, Executed, TransformedRequest, TransformedResponse, Validated};

impl<O: ClientOperation> Validated<O> {
    pub fn new(op: O) -> Result<Self, MicroserviceClientError> {
        let operation = std::any::type_name::<O>();
        op.validate().map_err(|err| {
            logger::warn!(operation, error = ?err, "microservice validation failed");
            err
        })?;
        Ok(Self(op))
    }

    pub fn into_transformed_request(self) -> Result<TransformedRequest<O>, MicroserviceClientError> {
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
    pub fn into_transformed_response(self) -> Result<TransformedResponse<O>, MicroserviceClientError> {
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
    client: &super::payment_method::PaymentMethodClient<'_>,
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

    headers.insert((
        header_name.to_string(),
        Maskable::Normal(trace_id),
    ));

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
