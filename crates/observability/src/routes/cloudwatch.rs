//! Handler for the CloudWatch evaluation route. The route tree that mounts it is in
//! [`crate::routes::app`].

use actix_web::{web, HttpRequest, HttpResponse};
use time::OffsetDateTime;

use crate::{auth, core, services, state::AppState, types::EvaluateResponse};

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
            let catalogue = match state.metrics.as_deref() {
                Some(provider) => {
                    core::cloudwatch::evaluate_catalogue(
                        provider,
                        &state.conf.cloudwatch,
                        OffsetDateTime::now_utc(),
                    )
                    .await
                }
                // Boot refuses a catalogue with no client, so there is nothing to evaluate.
                None => core::cloudwatch::Catalogue::default(),
            };

            Ok(EvaluateResponse::from(catalogue))
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}
