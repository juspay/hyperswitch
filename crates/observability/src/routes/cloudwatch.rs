//! Handlers for the CloudWatch routes. The route tree that mounts them is in
//! [`crate::routes::app`].

use actix_web::{web, HttpRequest, HttpResponse};
use common_utils::date_time;

use crate::{
    auth, core, services,
    state::AppState,
    types::{EvaluateResponse, NotifyResponse},
};

/// `GET /alerts/cloudwatch/evaluate`.
///
/// Reads every configured definition and answers with what each of its rules currently says. It
/// delivers nothing and remembers nothing, so two calls a second apart may disagree only because
/// the metrics did.
pub async fn evaluate(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move {
            let catalogue =
                core::cloudwatch::evaluate_catalogue(&state, date_time::now().assume_utc()).await;

            Ok(EvaluateResponse::from(catalogue))
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `POST /alerts/cloudwatch/notify`.
///
/// The same evaluation, followed by an announcement for every rule found breaching. `GET
/// /alerts/cloudwatch/evaluate` is the dry run: identical states, nothing sent.
pub async fn notify(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move {
            let announced = core::cloudwatch::announce::evaluate_and_announce(
                &state,
                date_time::now().assume_utc(),
            )
            .await;

            Ok(NotifyResponse::from(announced))
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}
