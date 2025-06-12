use actix_web::{web, Responder};
use error_stack::ResultExt;
use external_services::grpc_client::recovery_trainer_client;
use router_env::{instrument, logger, tracing, Flow};

use super::app::{ReqState, SessionState};
use crate::{
    core::{api_locking::LockAction, errors},
    services::{api, authentication as auth, ApplicationResponse},
    types::api::admin as admin_api_types,
};

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::TriggerTrainingJob))]
/// Handles an HTTP request to trigger a new training job via the gRPC trainer service.
///
/// Accepts a JSON payload describing the training job parameters, initiates the training job using the gRPC trainer client, and returns the result as a JSON response. Requires admin API authentication.
///
/// # Examples
///
/// ```
/// // Example Actix-web test for the endpoint
/// use actix_web::{test, web, App};
/// use recovery_trainer_client::TriggerTrainingRequest;
///
/// let app = test::init_service(
///     App::new().route(
///         "/trigger-training",
///         web::post().to(trigger_training_job),
///     ),
/// )
/// .await;
///
/// let req = test::TestRequest::post()
///     .uri("/trigger-training")
///     .set_json(&TriggerTrainingRequest {
///         model_version_tag: "v1.0".to_string(),
///         enable_incremental_learning: false,
///     })
///     .to_request();
///
/// let resp = test::call_service(&app, req).await;
/// assert!(resp.status().is_success());
/// ```
pub async fn trigger_training_job(
    state: web::Data<crate::AppState>,
    http_req: actix_web::HttpRequest,
    json_payload: web::Json<recovery_trainer_client::TriggerTrainingRequest>,
) -> impl Responder {
    let flow = Flow::TriggerTrainingJob;
    logger::debug!("Triggering training job endpoint called");
    let request_data = json_payload.into_inner();
    logger::debug!(deserialized_request = ?request_data, "Received and deserialized TriggerTrainingRequest");

    Box::pin(api::server_wrap(
        flow,
        state,
        &http_req,
        request_data,
        |state: SessionState, _auth: (), req_body , _req_state: ReqState| async move {
            logger::debug!("Inside trigger_training_job closure");
            let mut trainer_client = state.grpc_client.trainer_client.clone();
            let grpc_headers = state.get_grpc_headers();

            let model_version_tag = req_body.model_version_tag;
            let enable_incremental_learning = req_body.enable_incremental_learning;

            logger::debug!(%model_version_tag, %enable_incremental_learning, "Calling trainer_client.trigger_training");

            let response = trainer_client
                .get_training(
                    model_version_tag,
                    enable_incremental_learning,
                    grpc_headers,
                )
                .await
                .map_err(|err| {
                    logger::error!(grpc_error = ?err, "Trainer service TriggerTraining call failed");
                    errors::ApiErrorResponse::InternalServerError
                })
                .attach_printable("Trainer service TriggerTraining call failed")?;
            logger::debug!(?response, "Trainer service TriggerTraining call successful");

            Ok(ApplicationResponse::Json(response))
        },
        &auth::AdminApiAuth,
        LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::GetTrainingJobStatus))]
/// Retrieves the status of a training job by job ID.
///
/// This endpoint accepts a job ID as a path parameter, queries the external trainer service for the status of the corresponding training job, and returns the result as a JSON response. Requires admin API authentication.
///
/// # Examples
///
/// ```
/// // Example Actix-web route registration
/// app.route(
///     "/api/v2/recovery_trainer/job_status/{job_id}",
///     web::get().to(get_the_training_job_status),
/// );
/// ```
pub async fn get_the_training_job_status(
    state: web::Data<crate::AppState>,
    http_req: actix_web::HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    let flow = Flow::GetTrainingJobStatus;
    let job_id = path.into_inner();
    logger::debug!(%job_id, "Get training job status endpoint called");

    Box::pin(api::server_wrap(
        flow,
        state,
        &http_req,
        job_id.clone(),
        |state: SessionState, _auth: (), req_job_id: String, _req_state: ReqState| async move {
            logger::debug!(%req_job_id, "Inside get_training_job_status closure");
            let mut trainer_client = state.grpc_client.trainer_client.clone();
            let grpc_headers = state.get_grpc_headers();
            logger::debug!(%req_job_id, "Calling trainer_client.get_training_job_status");

            let response = trainer_client
                .get_the_training_job_status(req_job_id, grpc_headers)
                .await
                .map_err(|err| {
                    logger::error!(grpc_error = ?err, "Trainer service GetTrainingJobStatus call failed");
                    errors::ApiErrorResponse::InternalServerError
                })
                .attach_printable("Trainer service GetTrainingJobStatus call failed")?;

            logger::debug!(?response, "Trainer service GetTrainingJobStatus call successful");

            Ok(ApplicationResponse::Json(response))
        },
        &auth::AdminApiAuth,
        LockAction::NotApplicable,
    ))
    .await
}
