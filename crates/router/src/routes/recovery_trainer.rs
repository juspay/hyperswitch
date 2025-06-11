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