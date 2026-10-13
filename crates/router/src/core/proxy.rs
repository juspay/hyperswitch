use super::{
    errors::{self, RouterResponse, RouterResult},
    metrics,
};
use crate::{logger, routes::SessionState, services, types::domain};
pub mod utils;
use api_models::proxy as proxy_api_models;
use base64::{engine::general_purpose::STANDARD, Engine};
use common_utils::proxy::PreparedProxyBody;
use error_stack::ResultExt;
use external_services::http_client::outbound_destination;
use hyperswitch_interfaces::types::Response;
use hyperswitch_masking::PeekInterface;
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
    let proxy_record = req_wrapper
        .get_proxy_record(&state, platform.get_provider())
        .await?;

    let vault_data = proxy_record.get_vault_data(&state, platform).await?;

    let declared_format = req_wrapper
        .0
        .body
        .as_ref()
        .map_or("legacy", |body| body.peek().as_metric_str());
    let (body, content_type) = req_wrapper
        .get_content_type()
        .and_then(|content_type| {
            req_wrapper
                .0
                .connector_body(content_type)
                .map(|body| (body, content_type))
                .map_err(|err| {
                    errors::ApiErrorResponse::InvalidRequestData {
                        message: err.to_string(),
                    }
                    .into()
                })
        })
        .inspect_err(|_| {
            logger::warn!(
                body_format = declared_format,
                stage = "body_format",
                "Invalid proxy body format or Content-Type"
            );
            metrics::PROXY_BODY_PREPARATION_FAILURE.add(
                1,
                router_env::metric_attributes!(
                    ("body_format", declared_format),
                    ("stage", "body_format")
                ),
            );
        })?;
    let body_format = body.as_metric_str();
    let processed_body = body
        .prepare(&vault_data, content_type)
        .inspect_err(|_| {
            logger::warn!(
                body_format,
                stage = "body_preparation",
                "Failed to prepare proxy connector body"
            );
            metrics::PROXY_BODY_PREPARATION_FAILURE.add(
                1,
                router_env::metric_attributes!(
                    ("body_format", body_format),
                    ("stage", "body_preparation")
                ),
            );
        })
        .map_err(|err| errors::ApiErrorResponse::InvalidRequestData {
            message: err.to_string(),
        })?;

    let res = execute_proxy_request(&state, &req_wrapper, processed_body)
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

    let proxy_response = proxy_api_models::ProxyResponse::try_from(ProxyResponseWrapper(res))?;

    Ok(services::ApplicationResponse::Json(proxy_response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyperswitch_masking::ExposeInterface;

    #[test]
    fn saas_response_preserves_connector_bytes_status_and_repeated_headers() {
        for body in [
            &b" {\"result\":true}\n"[..],
            &b"<result/>"[..],
            &b"text response"[..],
            &b"\x00\xff\xfe"[..],
            &b""[..],
        ] {
            let mut headers = reqwest::header::HeaderMap::new();
            headers.append("set-cookie", "one=1".parse().unwrap());
            headers.append("set-cookie", "two=2".parse().unwrap());
            let response = Response {
                status_code: 422,
                headers: Some(headers),
                response: body.to_vec().into(),
            };
            let proxy =
                proxy_api_models::ProxyResponse::try_from(ProxyResponseWrapper(response)).unwrap();
            assert_eq!(proxy.status_code, 422);
            assert_eq!(
                STANDARD
                    .decode(proxy.response_body_base64.unwrap().expose())
                    .unwrap(),
                body
            );
            assert_eq!(proxy.response_headers_multi.unwrap().expose().len(), 2);
        }
    }

    #[test]
    fn non_ascii_connector_headers_do_not_fail_the_proxy_response() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.append(
            "x-note",
            reqwest::header::HeaderValue::from_bytes(b"caf\xe9").unwrap(),
        );
        let response = Response {
            status_code: 200,
            headers: Some(headers),
            response: b"{}".to_vec().into(),
        };
        let proxy =
            proxy_api_models::ProxyResponse::try_from(ProxyResponseWrapper(response)).unwrap();
        assert_eq!(
            proxy.response_headers_multi.unwrap().expose(),
            vec![("x-note".to_owned(), "caf\u{fffd}".to_owned())]
        );
    }
}

async fn execute_proxy_request(
    state: &SessionState,
    req_wrapper: &utils::ProxyRequestWrapper,
    processed_body: PreparedProxyBody,
) -> RouterResult<Response> {
    let mut request = processed_body.into_request(
        req_wrapper.get_destination_url(),
        req_wrapper.get_method(),
        req_wrapper.get_headers(),
    );
    request.certificate = req_wrapper.0.certificate.clone();
    request.certificate_key = req_wrapper.0.certificate_key.clone();
    request.ca_certificate = req_wrapper.0.ca_certificate.clone();

    let response = services::call_connector_api_for_proxy(state, request, None)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to call the destination");

    response
        .map(|inner| inner.unwrap_or_else(|err_res| err_res))
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error while receiving response")
}

struct ProxyResponseWrapper(Response);

impl TryFrom<ProxyResponseWrapper> for proxy_api_models::ProxyResponse {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(wrapper: ProxyResponseWrapper) -> Result<Self, Self::Error> {
        let res = wrapper.0;
        let response_body: Value = serde_json::from_slice(&res.response).unwrap_or_else(|_| {
            std::str::from_utf8(&res.response)
                .map(|text| Value::String(text.to_owned()))
                .unwrap_or(Value::Null)
        });
        let response_body_base64 = Some(STANDARD.encode(&res.response).into());

        let status_code = res.status_code;
        let response_headers = proxy_api_models::Headers::from_header_map(res.headers.as_ref());
        // The connector has already processed the request, so a non-ASCII header
        // value must not turn its reply into a proxy failure.
        let response_headers_multi = res.headers.as_ref().map(|headers| {
            headers
                .iter()
                .map(|(name, value)| {
                    let value = value.to_str().map(str::to_owned).unwrap_or_else(|_| {
                        logger::warn!(
                            header_name = %name,
                            "Connector response header value is not visible ASCII; converting lossily"
                        );
                        String::from_utf8_lossy(value.as_bytes()).into_owned()
                    });
                    (name.to_string(), value)
                })
                .collect::<Vec<_>>()
        });

        Ok(Self {
            response: response_body.into(),
            response_body_base64,
            response_headers_multi: response_headers_multi.map(Into::into),
            status_code,
            response_headers: response_headers.into(),
        })
    }
}
