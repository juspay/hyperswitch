use actix_web::{web, HttpRequest, HttpResponse};
use api_models::verify_connector::VerifyConnectorRequest;
use router_env::{instrument, tracing, Flow};

use super::AppState;
use crate::{
    core::{api_locking, verify_connector},
    services::{self, authentication as auth, authorization::permissions::Permission},
};

#[instrument(skip_all, fields(flow = ?Flow::VerifyPaymentConnector))]
/// Handles the verification of payment connector credentials by wrapping the verification process in a server and applying the necessary authentication and API locking mechanisms. 
pub async fn payment_connector_verify(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<VerifyConnectorRequest>,
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
