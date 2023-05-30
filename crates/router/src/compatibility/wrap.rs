use std::future::Future;

use actix_web::{HttpRequest, HttpResponse, Responder};
use common_utils::errors::ErrorSwitch;
use router_env::{instrument, tracing};
use serde::Serialize;

use crate::{
    core::errors::{self, RouterResult},
    routes::app::AppStateInfo,
    services::{self, api, authentication as auth, logger},
};

#[instrument(skip(request, payload, state, func, api_authentication))]
pub async fn compatibility_api_wrap<'a, 'b, A, U, T, Q, F, Fut, S, E>(
    state: &'b A,
    request: &'a HttpRequest,
    payload: T,
    func: F,
    api_authentication: &dyn auth::AuthenticateAndFetch<U, A>,
) -> HttpResponse
where
    F: Fn(&'b A, U, T) -> Fut,
    Fut: Future<Output = RouterResult<api::ApplicationResponse<Q>>>,
    Q: Serialize + std::fmt::Debug + 'a,
    S: TryFrom<Q> + Serialize,
    E: Serialize + error_stack::Context + actix_web::ResponseError + Clone,
    U: auth::AuthInfo,
    error_stack::Report<E>: services::EmbedError,
    errors::ApiErrorResponse: ErrorSwitch<E>,
    T: std::fmt::Debug,
    A: AppStateInfo,
{
    let resp: common_utils::errors::CustomResult<_, E> = api::server_wrap_util(
        &router_env::Flow::CompatibilityLayerRequest,
        state,
        request,
        payload,
        func,
        api_authentication,
    )
    .await;
    match resp {
        Ok(api::ApplicationResponse::ResponseWithCustomHeader(response, headers)) => {
            let mut final_response = handle_application_response::<Q, S>(request, *response);
            let inner_headers = final_response.headers_mut();
            headers
                .iter()
                .for_each(|(name, value)| inner_headers.append(name.to_owned(), value.to_owned()));
            final_response
        }
        Ok(response) => handle_application_response::<Q, S>(request, response),
        Err(error) => {
            logger::error!(api_response_error=?error);
            api::log_and_return_error_response(error)
        }
    }
}

pub fn handle_application_response<Q, S>(
    request: &HttpRequest,
    server_resp: api::ApplicationResponse<Q>,
) -> HttpResponse
where
    Q: Serialize + std::fmt::Debug,
    S: TryFrom<Q> + Serialize,
{
    match server_resp {
        api::ApplicationResponse::Json(router_resp) => {
            let pg_resp = S::try_from(router_resp);
            match pg_resp {
                Ok(pg_resp) => match serde_json::to_string(&pg_resp) {
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
        api::ApplicationResponse::StatusOk => api::http_response_ok(),
        api::ApplicationResponse::TextPlain(text) => api::http_response_plaintext(text),
        api::ApplicationResponse::FileData((file_data, content_type)) => {
            api::http_response_file_data(file_data, content_type)
        }
        api::ApplicationResponse::JsonForRedirection(response) => {
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
        api::ApplicationResponse::Form(redirection_data) => api::build_redirection_form(
            &redirection_data.redirect_form,
            redirection_data.payment_method_data,
            redirection_data.amount,
            redirection_data.currency,
        )
        .respond_to(request)
        .map_into_boxed_body(),
        api::ApplicationResponse::ResponseWithCustomHeader(_, _) => {
            api::log_and_return_error_response(
                errors::ApiErrorResponse::InvalidRequestData {
                    message: "Found header response inside a header response".to_string(),
                }
                .into(),
            )
        }
    }
}
