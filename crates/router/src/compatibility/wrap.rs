use std::{future::Future, time::Instant};

use actix_web::{HttpRequest, HttpResponse, Responder};
use common_utils::errors::{ErrorSwitch, CustomResult};
use router_env::{instrument, tracing, Tag};
use serde::Serialize;

use crate::{
    core::errors::{self},
    routes::{app::AppStateInfo, metrics},
    services::{self, api, authentication as auth, logger},
};

#[instrument(skip(request, payload, state, func, api_authentication))]
pub async fn compatibility_api_wrap<'a, 'b, A, U, T, Q, F, Fut, S, E, E2>(
    flow: impl router_env::types::FlowMetric,
    state: &'b A,
    request: &'a HttpRequest,
    payload: T,
    func: F,
    api_authentication: &dyn auth::AuthenticateAndFetch<U, A>,
) -> HttpResponse
where
    F: Fn(&'b A, U, T) -> Fut,
    Fut: Future<Output = CustomResult<api::ApplicationResponse<Q>, E2>>,
    E2: ErrorSwitch<E> + std::error::Error + Send + Sync +'static,
    Q: Serialize + std::fmt::Debug + 'a,
    S: TryFrom<Q> + Serialize,
    E: Serialize + error_stack::Context + actix_web::ResponseError + Clone,
    U: auth::AuthInfo,
    error_stack::Report<E>: services::EmbedError,
    errors::ApiErrorResponse: ErrorSwitch<E>,
    T: std::fmt::Debug,
    A: AppStateInfo,
{
    let request_method = request.method().as_str();
    let url_path = request.path();
    tracing::Span::current().record("request_method", request_method);
    tracing::Span::current().record("request_url_path", url_path);

    let start_instant = Instant::now();
    logger::info!(tag = ?Tag::BeginRequest, payload = ?payload);

    let res = match metrics::request::record_request_time_metric(
        api::server_wrap_util(&flow, state, request, payload, func, api_authentication),
        &flow,
    )
    .await
    .map(|response| {
        logger::info!(api_response =? response);
        response
    }) {
        Ok(api::ApplicationResponse::Json(response)) => {
            let response = S::try_from(response);
            match response {
                Ok(response) => match serde_json::to_string(&response) {
                    Ok(res) => api::http_response_json(res),
                    Err(_) => api::http_response_err(
                        r#"{
                                "error": {
                                    "message": "Error serializing response from connector"
                                }
                            }"#,
                    ),
                },
                Err(_) => api::http_response_err(
                    r#"{
                        "error": {
                            "message": "Error converting juspay response to stripe response"
                        }
                    }"#,
                ),
            }
        }
        Ok(api::ApplicationResponse::JsonWithHeaders((response, headers))) => {
            let response = S::try_from(response);
            match response {
                Ok(response) => match serde_json::to_string(&response) {
                    Ok(res) => api::http_response_json_with_headers(res, headers),
                    Err(_) => api::http_response_err(
                        r#"{
                                "error": {
                                    "message": "Error serializing response from connector"
                                }
                            }"#,
                    ),
                },
                Err(_) => api::http_response_err(
                    r#"{
                        "error": {
                            "message": "Error converting juspay response to stripe response"
                        }
                    }"#,
                ),
            }
        }
        Ok(api::ApplicationResponse::StatusOk) => api::http_response_ok(),
        Ok(api::ApplicationResponse::TextPlain(text)) => api::http_response_plaintext(text),
        Ok(api::ApplicationResponse::FileData((file_data, content_type))) => {
            api::http_response_file_data(file_data, content_type)
        }
        Ok(api::ApplicationResponse::JsonForRedirection(response)) => {
            match serde_json::to_string(&response) {
                Ok(res) => api::http_redirect_response(res, response),
                Err(_) => api::http_response_err(
                    r#"{
                    "error": {
                        "message": "Error serializing response from connector"
                    }
                }"#,
                ),
            }
        }
        Ok(api::ApplicationResponse::Form(redirection_data)) => api::build_redirection_form(
            &redirection_data.redirect_form,
            redirection_data.payment_method_data,
            redirection_data.amount,
            redirection_data.currency,
        )
        .respond_to(request)
        .map_into_boxed_body(),
        Err(error) => api::log_and_return_error_response(error),
    };

    let response_code = res.status().as_u16();
    let end_instant = Instant::now();
    let request_duration = end_instant.saturating_duration_since(start_instant);
    logger::info!(
        tag = ?Tag::EndRequest,
        status_code = response_code,
        time_taken_ms = request_duration.as_millis(),
    );

    res
}
