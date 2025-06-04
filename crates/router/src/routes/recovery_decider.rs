use actix_web::{web, Responder};
use common_utils::custom_serde::prost_timestamp::SerializableTimestamp; // Import for HttpDeciderRequest
use error_stack::ResultExt;
use external_services::grpc_client::recovery_decider_client::{
    self, DeciderRequest as GrpcDeciderRequest, // Alias the generated request
};
use router_env::{instrument, logger, tracing, Flow};

use super::app::{AppState, SessionState};
use crate::{
    core::{api_locking::LockAction, errors},
    services::{api, authentication as auth, ApplicationResponse},
    events::api_logs::ApiEventMetric,
};
use common_utils::events::ApiEventsType;

// Define a new struct for HTTP request deserialization
#[derive(Debug, serde::Deserialize, masking::Serialize)]
pub struct HttpDeciderRequest {
    pub first_error_message: String,
    pub billing_state: String,
    pub card_funding: String,
    pub card_network: String,
    pub card_issuer: String,
    pub start_time: Option<SerializableTimestamp>, // Use the wrapper for Serde
    pub end_time: Option<SerializableTimestamp>,   // Use the wrapper for Serde
}

// Implement ApiEventMetric for HttpDeciderRequest
impl ApiEventMetric for HttpDeciderRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::RecoveryDeciderShouldRetry))]
pub async fn call_decider(
    state: web::Data<AppState>,
    http_req: actix_web::HttpRequest,
    json_payload: web::Json<HttpDeciderRequest>,
) -> impl Responder {
    let flow = Flow::RecoveryDeciderShouldRetry;
    logger::debug!("Decide should retry endpoint called");
    let http_request_data = json_payload.into_inner();
    logger::debug!(deserialized_request = ?http_request_data, "Received and deserialized HttpDeciderRequest");

    Box::pin(api::server_wrap(
        flow,
        state,
        &http_req,
        http_request_data, // Pass HttpDeciderRequest to server_wrap
        |state: SessionState, _auth: (), http_req_data_inner: HttpDeciderRequest, _req_state| async move {
            // Convert HttpDeciderRequest to GrpcDeciderRequest inside the closure
            let grpc_request_data = GrpcDeciderRequest {
                first_error_message: http_req_data_inner.first_error_message,
                billing_state: http_req_data_inner.billing_state,
                card_funding: http_req_data_inner.card_funding,
                card_network: http_req_data_inner.card_network,
                card_issuer: http_req_data_inner.card_issuer,
                start_time: http_req_data_inner.start_time.map(Into::into), // Convert SerializableTimestamp to prost_types::Timestamp
                end_time: http_req_data_inner.end_time.map(Into::into),     // Convert SerializableTimestamp to prost_types::Timestamp
            };

            let mut decider_client = state.grpc_client.recovery_decider_client.clone();
            let grpc_headers = state.get_grpc_headers();

            let response = decider_client
                .decide_on_retry(
                    grpc_request_data.first_error_message,
                    grpc_request_data.billing_state,
                    grpc_request_data.card_funding,
                    grpc_request_data.card_network,
                    grpc_request_data.card_issuer,
                    grpc_request_data.start_time,
                    grpc_request_data.end_time,
                    grpc_headers,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Decider gRPC call failed")?;

            Ok(ApplicationResponse::Json(response))
        },
        &auth::AdminApiAuth,
        LockAction::NotApplicable,
    ))
    .await
}
