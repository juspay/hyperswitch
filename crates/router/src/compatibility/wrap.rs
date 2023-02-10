use std::future::Future;

use actix_web::{HttpRequest, HttpResponse, Responder};
use error_stack::report;
use router_env::{instrument, tracing};
use serde::Serialize;

use crate::{
    core::errors::{self, RouterResult},
    routes::app::AppStateInfo,
    services::{api, authentication as auth, logger},
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
    S: From<Q> + Serialize,
    E: From<errors::ApiErrorResponse> + Serialize + error_stack::Context + actix_web::ResponseError,
    T: std::fmt::Debug,
    A: AppStateInfo,
{
    let resp = compatibility_wrap_util(state, request, payload, func, api_authentication).await;
    match resp {
        Ok(api::ApplicationResponse::Json(router_resp)) => {
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
        Ok(api::ApplicationResponse::StatusOk) => api::http_response_ok(),
        Ok(api::ApplicationResponse::TextPlain(text)) => api::http_response_plaintext(text),
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
        Ok(api::ApplicationResponse::Form(form_data)) => api::build_redirection_form(&form_data)
            .respond_to(request)
            .map_into_boxed_body(),
        Err(error) => {
            logger::error!(api_response_error=?error);
            let pg_error = E::from(error.current_context().clone());
            api::log_and_return_error_response(report!(pg_error))
        }
    }
}

#[instrument(skip(request, payload, state, func, api_auth))]
pub async fn compatibility_wrap_util<'a, 'b, A, U, T, Q, F, Fut>(
    state: &'b A,
    request: &'a HttpRequest,
    payload: T,
    func: F,
    api_auth: &dyn auth::AuthenticateAndFetch<U, A>,
) -> RouterResult<api::ApplicationResponse<Q>>
where
    F: Fn(&'b A, U, T) -> Fut,
    Fut: Future<Output = RouterResult<api::ApplicationResponse<Q>>>,
    Q: Serialize + std::fmt::Debug + 'a,
    T: std::fmt::Debug,
    A: AppStateInfo,
{
    let auth_out = api_auth
        .authenticate_and_fetch(request.headers(), state)
        .await?;
    func(state, auth_out, payload).await
}
