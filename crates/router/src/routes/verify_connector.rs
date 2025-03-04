#[cfg(feature = "v1")]
use actix_web::{web, HttpRequest, HttpResponse};
#[cfg(feature = "v1")]
use api_models::verify_connector::VerifyConnectorRequest;
#[cfg(feature = "v1")]
use router_env::{instrument, tracing, Flow};

#[cfg(feature = "v1")]
use super::AppState;
#[cfg(feature = "v1")]
use crate::{
    core::{api_locking, verify_connector},
    services::{self, authentication as auth, authorization::permissions::Permission},
};

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::VerifyPaymentConnector))]
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
        |state, auth: auth::AuthenticationData, req, _| {
            verify_connector::verify_connector_credentials(state, req, auth.profile_id)
        },
        &auth::JWTAuth {
            permission: Permission::MerchantConnectorWrite,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
