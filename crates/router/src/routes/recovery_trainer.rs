#![cfg(all(feature = "v2", feature = "revenue_recovery"))]
use actix_web::{web, Responder};
use error_stack::ResultExt;
use external_services::grpc_client::{TrainerClientInterface, TriggerTrainingRequest};
use router_env::{instrument, logger, tracing, Flow};

use super::app::{ReqState, SessionState};
use crate::{
    core::{api_locking::LockAction, errors},
    services::{api, authentication as auth, ApplicationResponse},
    types::api::admin as admin_api_types,
};

#[instrument(skip_all, fields(flow = ?Flow::TriggerTrainingJob))]
pub async fn trigger_training_job(
    state: web::Data<crate::AppState>,
    http_req: actix_web::HttpRequest,
    json_payload: web::Json<TriggerTrainingRequest>,
) -> impl Responder {
    let flow = Flow::TriggerTrainingJob;
    logger::debug!("Triggering training job endpoint called");
    let request_data = json_payload.into_inner();
    logger::debug!(deserialized_request = ?request_data, "Received and deserialized TriggerTrainingRequest");

    let headers = http_req.headers();
    let merchant_id_header = auth::HeaderMapStruct::new(headers);

    let merchant_id = match merchant_id_header.get_merchant_id_from_header() {
        Ok(merchant_id) => merchant_id,
        Err(e) => return api::log_and_return_error_response(e),
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &http_req,
        request_data,
        |session_state, _auth: (), req_body , _req_state| async move {
            logger::debug!("Inside trigger_training_job closure");
            let grpc_clients_arc = session_state.grpc_client.clone();
            let client_ref_in_once_cell = grpc_clients_arc.trainer_client_cell.get_or_try_init(|| async {
                logger::info!("Attempting to initialize gRPC trainer client");
                grpc_clients_arc.trainer_config
                    .get_trainer_service_client(grpc_clients_arc.hyper_client_for_trainer.clone())
                    .map(|client| -> Box<dyn TrainerClientInterface> { Box::new(client) })
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .map_err(|report| {
                        logger::error!(trainer_client_init_error = ?report, "Failed to initialize trainer gRPC client");
                        report.current_context().clone()
                    })
            }).await?;

            let mut trainer_client = dyn_clone::clone_box(&**client_ref_in_once_cell);

            let model_version_tag = req_body.model_version_tag;

            logger::debug!(%model_version_tag, %merchant_id , "Calling trainer_client.trigger_training");

            let response = trainer_client
                .get_training(
                    model_version_tag,
                    merchant_id.into(),
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
        |session_state, _auth: (), req_job_id: String, _req_state| async move {
            logger::debug!(%req_job_id, "Inside get_training_job_status closure");
            let grpc_clients_arc = session_state.grpc_client.clone();
            let client_ref_in_once_cell = grpc_clients_arc.trainer_client_cell.get_or_try_init(|| async {
                logger::info!("Attempting to initialize gRPC trainer client for get_status");
                grpc_clients_arc.trainer_config
                    .get_trainer_service_client(grpc_clients_arc.hyper_client_for_trainer.clone())
                    .map(|client| -> Box<dyn TrainerClientInterface> { Box::new(client) })
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .map_err(|report| {
                        logger::error!(trainer_client_init_error = ?report, "Failed to initialize trainer gRPC client for get_status");
                        report.current_context().clone()
                    })
            }).await?;

            let mut trainer_client = dyn_clone::clone_box(&**client_ref_in_once_cell);
            logger::debug!(%req_job_id, "Calling trainer_client.get_training_job_status");

            let response = trainer_client
                .get_the_training_job_status(req_job_id)
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
