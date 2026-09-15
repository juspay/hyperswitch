//! Handlers for the CloudWatch routes. The route tree that mounts them is in
//! [`crate::routes::app`].

use actix_web::{web, HttpRequest, HttpResponse};
use common_utils::date_time;

use crate::{auth, core, services, state::AppState, types::EvaluateResponse};

/// `GET /alerts/cloudwatch/evaluate`.
///
/// The dry run. Evaluates the catalogue twice, reports what each rule says and what changed since
/// the previous evaluation, renders the messages those changes would produce — and sends none of
/// them.
pub async fn evaluate(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    announce(state, request, false).await
}

/// `POST /alerts/cloudwatch/notify`.
///
/// The same evaluation, delivered. Only rules that changed state are announced, so a breach that
/// stays breaching is reported once rather than on every call.
pub async fn notify(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    announce(state, request, true).await
}

async fn announce(state: web::Data<AppState>, request: HttpRequest, deliver: bool) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move {
            let announced = core::cloudwatch::announce::evaluate_and_announce(
                &state,
                date_time::now().assume_utc(),
                deliver,
            )
            .await;

            Ok(EvaluateResponse::from(announced))
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}
