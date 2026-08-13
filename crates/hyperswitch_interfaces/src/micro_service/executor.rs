use std::time::Instant;

use common_utils::{
    consts::{TENANT_HEADER, X_MERCHANT_ID},
    request::{Headers, Request},
};
use hyperswitch_masking::Maskable;
use router_env::{logger, RequestIdentifier};
use url::Url;

use super::{
    error::{MicroserviceClientError, MicroserviceClientErrorKind},
    state::{ClientOperation, Executed, TransformedRequest, TransformedResponse, Validated},
    MicroserviceClient,
};
use crate::{
    api_client::{call_connector_api, ApiClientWrapper},
    events::microservice_api_logs::MicroserviceEvent,
};

/// Profile id header forwarded to microservices; matched case-insensitively.
const X_PROFILE_ID: &str = "x-profile-id";

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
            v1_request: self.request,
            request,
        })
    }
}

/// Read an identifying header (e.g. `x-merchant-id`) out of the parent headers so the
/// emitted API event carries it.
fn get_header_value(headers: &Headers, name: &str) -> Option<String> {
    headers.iter().find_map(|(key, value)| {
        key.eq_ignore_ascii_case(name)
            .then(|| value.clone().into_inner())
    })
}

impl<O: ClientOperation> TransformedRequest<O> {
    /// Execute the HTTP call for this operation and capture the raw response payload.
    pub async fn execute(
        self,
        state: &dyn ApiClientWrapper,
        base_url: &Url,
        parent_headers: Headers,
        trace_header: &RequestIdentifier,
        service_name: &str,
    ) -> Result<Executed<O>, MicroserviceClientError> {
        let operation = std::any::type_name::<O>();
        // Step 1: Build path and URL.
        let path = {
            let mut path = O::PATH_TEMPLATE.to_string();
            for (key, value) in self.op.path_params(&self.v1_request) {
                let token = format!("{{{key}}}");
                path = path.replace(&token, &value);
            }
            path
        };
        let mut url = base_url.join(&path).map_err(|e| {
            logger::error!(operation, error = ?e, "microservice URL join failed");
            MicroserviceClientError {
                operation: operation.to_string(),
                kind: MicroserviceClientErrorKind::Transport(format!(
                    "Failed to construct URL: {e}"
                )),
            }
        })?;
        let query_params = self.op.query_params(&self.v1_request);
        if !query_params.is_empty() {
            let mut query = url.query_pairs_mut();
            for (key, value) in query_params {
                query.append_pair(key, &value);
            }
        }

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

        // Step 3: Attach body (if any), capturing its masked form for the API event first.
        let masked_request_body = self.op.body(self.request).map(|body| {
            let masked = match &body {
                common_utils::request::RequestContent::Json(inner)
                | common_utils::request::RequestContent::FormUrlEncoded(inner) => inner
                    .masked_serialize()
                    .unwrap_or_else(|err| serde_json::json!({"error": err.to_string()})),
                _ => serde_json::json!({"request": "non-json body, not captured"}),
            };
            http_request.set_body(body);
            masked
        });

        // Step 4: Build the API event for this call. Merchant/profile identifiers travel in
        // the parent headers, so read them back from there.
        let mut microservice_event = MicroserviceEvent::new(
            state.get_tenant().tenant_id.clone(),
            service_name.to_string(),
            operation,
            masked_request_body,
            url.to_string(),
            O::METHOD,
            get_header_value(&http_request.headers, X_MERCHANT_ID),
            get_header_value(&http_request.headers, X_PROFILE_ID),
            state.get_request_id().as_ref(),
        );
        let start_time = Instant::now();

        // Step 5: Execute request and decode response.
        let response = call_connector_api(state, http_request, operation, None).await;
        microservice_event.set_latency(start_time.elapsed().as_millis());

        let result = match response {
            Err(e) => {
                logger::error!(operation, error = ?e, "microservice request failed");
                microservice_event.set_error_string(format!("Connector API error: {e}"));
                Err(MicroserviceClientError {
                    operation: operation.to_string(),
                    kind: MicroserviceClientErrorKind::Transport(format!(
                        "Connector API error: {e}"
                    )),
                })
            }
            Ok(Ok(success)) => {
                microservice_event.set_status_code(success.status_code);
                serde_json::from_slice(&success.response).map_err(|e| {
                    logger::error!(
                        operation,
                        error = ?e,
                        "microservice response decode failed"
                    );
                    microservice_event.set_error_string(format!("Failed to parse response: {e}"));
                    MicroserviceClientError {
                        operation: operation.to_string(),
                        kind: MicroserviceClientErrorKind::Deserialize(format!(
                            "Failed to parse response: {e}"
                        )),
                    }
                })
            }
            Ok(Err(err_resp)) => {
                logger::warn!(
                    operation,
                    status = err_resp.status_code,
                    "microservice upstream error"
                );
                microservice_event.set_status_code(err_resp.status_code);
                let body = String::from_utf8_lossy(&err_resp.response);
                let truncated_body: String = body.chars().take(500).collect();
                microservice_event.set_error_string(truncated_body.clone());
                Err(MicroserviceClientError {
                    operation: operation.to_string(),
                    kind: MicroserviceClientErrorKind::Upstream {
                        status: err_resp.status_code,
                        body: truncated_body,
                    },
                })
            }
        };

        // The response body can carry raw payment method data, which must never reach the
        // event pipe unmasked — so success events record status/latency only.
        state
            .event_handler()
            .log_microservice_event(&microservice_event);

        result.map(|response| Executed {
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
    client: &impl MicroserviceClient,
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
            client.service_name(),
        )
        .await?;
    Ok(executed.into_transformed_response()?.output)
}
