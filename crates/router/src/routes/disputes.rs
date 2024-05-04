use actix_multipart::Multipart;
use actix_web::{web, HttpRequest, HttpResponse};
use api_models::disputes as dispute_models;
use router_env::{instrument, tracing, Flow};

use crate::{core::api_locking, services::authorization::permissions::Permission};
pub mod utils;

use super::app::AppState;
use crate::{
    core::disputes,
    services::{api, authentication as auth},
    types::api::disputes as dispute_types,
};

/// Disputes - Retrieve Dispute
#[utoipa::path(
    get,
    path = "/disputes/{dispute_id}",
    params(
        ("dispute_id" = String, Path, description = "The identifier for dispute")
    ),
    responses(
        (status = 200, description = "The dispute was retrieved successfully", body = DisputeResponse),
        (status = 404, description = "Dispute does not exist in our records")
    ),
    tag = "Disputes",
    operation_id = "Retrieve a Dispute",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::DisputesRetrieve))]
pub async fn retrieve_dispute(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::DisputesRetrieve;
    let dispute_id = dispute_types::DisputeId {
        dispute_id: path.into_inner(),
    };
    api::server_wrap(
        flow,
        state,
        &req,
        dispute_id,
        |state, auth, req| disputes::retrieve_dispute(state, auth.merchant_account, req),
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::DisputeRead),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}
/// Disputes - List Disputes
#[utoipa::path(
    get,
    path = "/disputes/list",
    params(
        ("limit" = Option<i64>, Query, description = "The maximum number of Dispute Objects to include in the response"),
        ("dispute_status" = Option<DisputeStatus>, Query, description = "The status of dispute"),
        ("dispute_stage" = Option<DisputeStage>, Query, description = "The stage of dispute"),
        ("reason" = Option<String>, Query, description = "The reason for dispute"),
        ("connector" = Option<String>, Query, description = "The connector linked to dispute"),
        ("received_time" = Option<PrimitiveDateTime>, Query, description = "The time at which dispute is received"),
        ("received_time.lt" = Option<PrimitiveDateTime>, Query, description = "Time less than the dispute received time"),
        ("received_time.gt" = Option<PrimitiveDateTime>, Query, description = "Time greater than the dispute received time"),
        ("received_time.lte" = Option<PrimitiveDateTime>, Query, description = "Time less than or equals to the dispute received time"),
        ("received_time.gte" = Option<PrimitiveDateTime>, Query, description = "Time greater than or equals to the dispute received time"),
    ),
    responses(
        (status = 200, description = "The dispute list was retrieved successfully", body = Vec<DisputeResponse>),
        (status = 401, description = "Unauthorized request")
    ),
    tag = "Disputes",
    operation_id = "List Disputes",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::DisputesList))]
pub async fn retrieve_disputes_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Query<dispute_models::DisputeListConstraints>,
) -> HttpResponse {
    let flow = Flow::DisputesList;
    let payload = payload.into_inner();
    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, req| disputes::retrieve_disputes_list(state, auth.merchant_account, req),
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::DisputeRead),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}
/// Disputes - Accept Dispute
#[utoipa::path(
    get,
    path = "/disputes/accept/{dispute_id}",
    params(
        ("dispute_id" = String, Path, description = "The identifier for dispute")
    ),
    responses(
        (status = 200, description = "The dispute was accepted successfully", body = DisputeResponse),
        (status = 404, description = "Dispute does not exist in our records")
    ),
    tag = "Disputes",
    operation_id = "Accept a Dispute",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::DisputesRetrieve))]
pub async fn accept_dispute(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::DisputesRetrieve;
    let dispute_id = dispute_types::DisputeId {
        dispute_id: path.into_inner(),
    };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        dispute_id,
        |state, auth, req| {
            disputes::accept_dispute(state, auth.merchant_account, auth.key_store, req)
        },
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::DisputeWrite),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
/// Disputes - Submit Dispute Evidence
#[utoipa::path(
    post,
    path = "/disputes/evidence",
    request_body=AcceptDisputeRequestData,
    responses(
        (status = 200, description = "The dispute evidence submitted successfully", body = AcceptDisputeResponse),
        (status = 404, description = "Dispute does not exist in our records")
    ),
    tag = "Disputes",
    operation_id = "Submit Dispute Evidence",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::DisputesEvidenceSubmit))]
pub async fn submit_dispute_evidence(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<dispute_models::SubmitEvidenceRequest>,
) -> HttpResponse {
    let flow = Flow::DisputesEvidenceSubmit;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth, req| {
            disputes::submit_evidence(state, auth.merchant_account, auth.key_store, req)
        },
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::DisputeWrite),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
/// Disputes - Attach Evidence to Dispute
///
/// To attach an evidence file to dispute
#[utoipa::path(
    put,
    path = "/disputes/evidence",
    request_body=MultipartRequestWithFile,
    responses(
        (status = 200, description = "Evidence attached to dispute", body = CreateFileResponse),
        (status = 400, description = "Bad Request")
    ),
    tag = "Disputes",
    operation_id = "Attach Evidence to Dispute",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::AttachDisputeEvidence))]
pub async fn attach_dispute_evidence(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: Multipart,
) -> HttpResponse {
    let flow = Flow::AttachDisputeEvidence;
    //Get attach_evidence_request from the multipart request
    let attach_evidence_request_result = utils::get_attach_evidence_request(payload).await;
    let attach_evidence_request = match attach_evidence_request_result {
        Ok(valid_request) => valid_request,
        Err(err) => return api::log_and_return_error_response(err),
    };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        attach_evidence_request,
        |state, auth, req| {
            disputes::attach_evidence(state, auth.merchant_account, auth.key_store, req)
        },
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::DisputeWrite),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Disputes - Retrieve Dispute
#[utoipa::path(
    get,
    path = "/disputes/evidence/{dispute_id}",
    params(
        ("dispute_id" = String, Path, description = "The identifier for dispute")
    ),
    responses(
        (status = 200, description = "The dispute evidence was retrieved successfully", body = DisputeResponse),
        (status = 404, description = "Dispute does not exist in our records")
    ),
    tag = "Disputes",
    operation_id = "Retrieve a Dispute Evidence",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::RetrieveDisputeEvidence))]
pub async fn retrieve_dispute_evidence(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::RetrieveDisputeEvidence;
    let dispute_id = dispute_types::DisputeId {
        dispute_id: path.into_inner(),
    };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        dispute_id,
        |state, auth, req| disputes::retrieve_dispute_evidence(state, auth.merchant_account, req),
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::DisputeRead),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Disputes - Delete Evidence attached to a Dispute
///
/// To delete an evidence file attached to a dispute
#[utoipa::path(
    put,
    path = "/disputes/evidence",
    request_body=DeleteEvidenceRequest,
    responses(
        (status = 200, description = "Evidence deleted from a dispute"),
        (status = 400, description = "Bad Request")
    ),
    tag = "Disputes",
    operation_id = "Delete Evidence attached to a Dispute",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::DeleteDisputeEvidence))]
pub async fn delete_dispute_evidence(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<dispute_models::DeleteEvidenceRequest>,
) -> HttpResponse {
    let flow = Flow::DeleteDisputeEvidence;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth, req| disputes::delete_evidence(state, auth.merchant_account, req),
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::DisputeWrite),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
