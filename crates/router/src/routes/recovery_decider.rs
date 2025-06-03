use actix_web::{web, Responder};
use error_stack::ResultExt;
use external_services::grpc_client::recovery_decider_client;
use router_env::{instrument, logger, tracing, Flow};

use super::app::{AppState, SessionState};
use crate::{
    core::{api_locking::LockAction, errors},
    services::{api, authentication as auth, ApplicationResponse},
    types::api::admin as admin_api_types,
};

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::RecoveryDeciderShouldRetry))]
pub async fn recovery_should_retry_test(
    state: web::Data<AppState>,
    http_req: actix_web::HttpRequest,
    json_payload: web::Json<recovery_decider_client::RecoveryDeciderRequest>,
) -> impl Responder {
    let flow = Flow::RecoveryDeciderShouldRetry;
    logger::debug!("Should retry endpoint called");
    let request_data = json_payload.into_inner();
    logger::debug!(deserialized_request = ?request_data, "Received and deserialized RecoveryDeciderRequest");

    Box::pin(api::server_wrap(
        flow,
        state,
        &http_req,
        request_data,
        |state, _auth: (), request_data, _req_state| async move {
            use error_stack::ResultExt;
            use external_services::grpc_client::recovery_decider_client::RecoveryDeciderRequest;

            use crate::services::ApplicationResponse;

            let mut recovery_client = state.grpc_client.recovery_trainer_client.clone();

            let grpc_headers = state.get_grpc_headers();

            let response = recovery_client
                .get_decider(
                    request_data.first_error_message,
                    request_data.billing_state,
                    request_data.card_funding,
                    request_data.card_network,
                    request_data.card_issuer,
                    request_data.txn_time,
                    grpc_headers,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Recovery decider gRPC call failed")?;

            Ok(ApplicationResponse::Json(response))
        },
        &auth::AdminApiAuth,
        LockAction::NotApplicable,
    ))
    .await
}
