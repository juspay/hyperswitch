use super::errors::{self, RouterResponse, RouterResult};
use crate::{logger, routes::SessionState, services, types::domain};
mod body;
mod transform;
pub mod utils;
use api_models::proxy as proxy_api_models;
use common_utils::fp_utils::when;
use error_stack::ResultExt;
use external_services::http_client::outbound_destination;
use hyperswitch_interfaces::types::Response;

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

    let request = match req_wrapper.get_method() {
        // GET and DELETE send no body: the token is checked, but there is nothing to validate
        // or fill with vault data, so the request is forwarded as it is.
        method @ (services::Method::Get | services::Method::Delete) => {
            when(req_wrapper.0.transformation.is_some(), || {
                Err(errors::ApiErrorResponse::InvalidRequestData {
                    message: "Transformation requires a request body".to_string(),
                })
            })?;
            req_wrapper
                .get_proxy_record(&state, platform.get_provider())
                .await?;
            services::RequestBuilder::new()
                .method(method)
                .url(req_wrapper.get_destination_url())
                .attach_default_headers()
                .headers(req_wrapper.get_headers())
                .build()
        }
        _ => body_request(&state, platform, &req_wrapper).await?,
    };

    let res = execute_proxy_request(&state, request).await?;

    let proxy_response = proxy_api_models::ProxyResponse::try_from(ProxyResponseWrapper(res))?;

    Ok(services::ApplicationResponse::Json(proxy_response))
}

/// The connector request for a method with a body. The body and any transformation are
/// validated before the vault is read: reading the vault consumes the single-use CVC, so an
/// invalid request must not reach it. Only a request with transformation instructions is
/// transformed.
async fn body_request(
    state: &SessionState,
    platform: domain::Platform,
    req_wrapper: &utils::ProxyRequestWrapper,
) -> RouterResult<services::Request> {
    let content_type = req_wrapper.get_content_type()?;
    let body = body::ProxyBody::new(req_wrapper.0.request_body.clone(), content_type.as_ref())
        .map_err(invalid_body)?;
    let validated_body = body
        .builder()
        .validate(content_type.as_ref())
        .map_err(invalid_body)?;
    let plan = req_wrapper
        .0
        .transformation
        .as_ref()
        .map(|transformation| {
            transform::ValidatedPlan::new(transformation, &body, req_wrapper.0.headers.as_map())
        })
        .transpose()
        .map_err(|error| invalid_body(error.into()))?;

    let proxy_record = req_wrapper
        .get_proxy_record(state, platform.get_provider())
        .await?;

    let vault_data = proxy_record.get_vault_data(state, platform).await?;

    let url = req_wrapper.get_destination_url();
    let method = req_wrapper.get_method();
    let headers = req_wrapper.get_headers();
    let request = match plan {
        None => validated_body
            .build(&vault_data, url, method, headers)
            .map_err(invalid_body)?,
        Some(plan) => validated_body
            .transform(&vault_data, &plan)
            .map_err(invalid_body)?
            .build(url, method, headers),
    }
    .into_inner();
    Ok(request)
}

/// A connector body that cannot be sent is a client error.
fn invalid_body(error: body::ProxyBodyError) -> errors::ApiErrorResponse {
    errors::ApiErrorResponse::InvalidRequestData {
        message: error.to_string(),
    }
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

impl TryFrom<ProxyResponseWrapper> for proxy_api_models::ProxyResponse {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(wrapper: ProxyResponseWrapper) -> Result<Self, Self::Error> {
        let res = wrapper.0;
        Ok(Self {
            response: response_body(&res.response)?,
            status_code: res.status_code,
            response_headers: proxy_api_models::Headers::from_header_map(res.headers.as_ref()),
        })
    }
}

/// The connector body as JSON, else as text. Only UTF-8 text is supported; other encodings
/// are rejected rather than converted lossily.
fn response_body(body: &[u8]) -> RouterResult<proxy_api_models::ProxyResponseBody> {
    serde_json::from_slice(body)
        .map(proxy_api_models::ProxyResponseBody::Json)
        .or_else(|_| {
            std::str::from_utf8(body)
                .map(|text| proxy_api_models::ProxyResponseBody::Text(text.to_owned()))
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Connector response is neither JSON nor UTF-8 text")
        })
}
