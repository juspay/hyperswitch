use super::{
    errors::{self, RouterResponse, RouterResult},
    metrics,
};
use crate::{logger, routes::SessionState, services, types::domain};
mod body;
pub mod utils;
use api_models::proxy as proxy_api_models;
use base64::{engine::general_purpose::STANDARD, Engine};
use error_stack::ResultExt;
use external_services::http_client::outbound_destination;
use hyperswitch_interfaces::types::Response;
use serde_json::Value;

pub async fn proxy_core(
    state: SessionState,
    platform: domain::Platform,
    req: proxy_api_models::ProxyRequest,
) -> RouterResponse<proxy_api_models::ProxyResponse> {
    outbound_destination::validate_destination(
        &req.destination_url,
        state.conf.proxy.bypass_proxy_hosts.as_deref(),
    )
    .await
    .change_context(errors::ApiErrorResponse::InvalidRequestData {
        message: "destination_url is not valid".to_string(),
    })?;

    let req_wrapper = utils::ProxyRequestWrapper(req);

    // Validate the connector body before the vault is read: reading the vault consumes
    // the single-use CVC, so an invalid request must not reach it.
    let content_type = req_wrapper.get_content_type()?;
    let body = body::ProxyBody::new(req_wrapper.0.request_body.clone(), content_type.as_ref())
        .inspect_err(|_| record_body_failure("unknown", "content_type"))
        .map_err(invalid_body)?;
    let body_format = body.as_metric_str();
    let validated_body = body
        .builder()
        .validate(content_type.as_ref())
        .inspect_err(|_| record_body_failure(body_format, "body_validation"))
        .map_err(invalid_body)?;

    let proxy_record = req_wrapper
        .get_proxy_record(&state, platform.get_provider())
        .await?;

    let vault_data = proxy_record.get_vault_data(&state, platform).await?;

    let request = validated_body
        .build(
            &vault_data,
            req_wrapper.get_destination_url(),
            req_wrapper.get_method(),
            req_wrapper.get_headers(),
        )
        .inspect_err(|_| record_body_failure(body_format, "body_preparation"))
        .map_err(invalid_body)?
        .into_inner();

    let res = execute_proxy_request(&state, request)
        .await
        .inspect_err(|_| {
            logger::error!(
                body_format,
                stage = "connector_request",
                "Proxy connector request failed"
            );
        })?;
    metrics::PROXY_CONNECTOR_RESPONSE_COUNT.add(
        1,
        router_env::metric_attributes!(
            ("body_format", body_format),
            ("status_code", res.status_code.to_string())
        ),
    );
    logger::info!(
        body_format,
        status_code = res.status_code,
        response_size = res.response.len(),
        "Proxy connector response received"
    );

    let proxy_response = proxy_api_models::ProxyResponse::from(ProxyResponseWrapper(res));

    Ok(services::ApplicationResponse::Json(proxy_response))
}

/// A connector body that cannot be sent is a client error.
fn invalid_body(error: body::ProxyBodyError) -> errors::ApiErrorResponse {
    errors::ApiErrorResponse::InvalidRequestData {
        message: error.to_string(),
    }
}

/// Log and count a connector body that cannot be sent, by format and failing stage.
fn record_body_failure(body_format: &'static str, stage: &'static str) {
    logger::warn!(body_format, stage, "Invalid proxy connector body");
    metrics::PROXY_BODY_PREPARATION_FAILURE.add(
        1,
        router_env::metric_attributes!(("body_format", body_format), ("stage", stage)),
    );
}

async fn execute_proxy_request(
    state: &SessionState,
    request: services::Request,
) -> RouterResult<Response> {
    let response = services::call_connector_api(state, request, "proxy", None)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to call the destination");

    response
        .map(|inner| match inner {
            Err(err_res) => {
                logger::error!("Error while receiving response: {err_res:?}");
                err_res
            }
            Ok(res) => res,
        })
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error while receiving response")
}

struct ProxyResponseWrapper(Response);

impl From<ProxyResponseWrapper> for proxy_api_models::ProxyResponse {
    fn from(wrapper: ProxyResponseWrapper) -> Self {
        let res = wrapper.0;
        Self {
            response_body_base64: Some(STANDARD.encode(&res.response)),
            response: response_value(&res.response),
            status_code: res.status_code,
            response_headers: proxy_api_models::Headers::from_header_map(res.headers.as_ref()),
        }
    }
}

/// The connector body as JSON, else as UTF-8 text, else `null`. The exact bytes always
/// travel in `response_body_base64`.
fn response_value(body: &[u8]) -> Value {
    serde_json::from_slice(body).unwrap_or_else(|_| {
        std::str::from_utf8(body).map_or(Value::Null, |text| Value::String(text.to_owned()))
    })
}
