use super::AppState;
use crate::{
    core::{api_locking, verify_connector},
    services::{self, authentication as auth, authorization::permissions::Permission},
};
use actix_web::{web, HttpRequest, HttpResponse};
use api_models::admin::MerchantConnectorCreate;
use router_env::{instrument, tracing, Flow};

#[instrument(skip_all, fields(flow = ?Flow::VerifyPaymentConnector))]
pub async fn payment_connector_verify(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<MerchantConnectorCreate>,
) -> HttpResponse {
    let flow = Flow::VerifyPaymentConnector;
    Box::pin(services::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, _: (), req| verify_connector::verify_connector_credentials(state, req),
        &auth::JWTAuth(Permission::MerchantConnectorAccountWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
