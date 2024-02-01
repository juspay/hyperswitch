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
/// Asynchronously retrieves a dispute using the provided state, request, and path. It creates a Flow for DisputesRetrieve, constructs a DisputeId from the provided path, and then calls the server_wrap method from the api module with the necessary parameters to retrieve the dispute. It awaits the result and returns an HttpResponse.
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
/// Retrieves a list of disputes based on the provided constraints. This method is an asynchronous function
/// that takes the application state, HTTP request, and query payload as input parameters. It then wraps the
/// retrieval process using the server_wrap method, which handles authentication, authorization, and API locking.
/// Finally, it awaits the result of the wrapped retrieval process and returns an HTTP response.
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
/// Asynchronously accepts a dispute by sending a request to the server, with the provided state, request, and path information. It creates a flow for DisputesRetrieve, extracts the dispute ID from the path, and uses it to call the server_wrap function. The server_wrap function handles the API call for accepting a dispute, along with the necessary authentication and locking actions, and returns the HTTP response.
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
/// Handles the submission of evidence for a dispute. This method takes in the current application state,
/// the HTTP request, and the JSON payload containing the evidence to be submitted. It sends the evidence
/// submission request to the disputes module after performing necessary authentication and authorization
/// checks. The method returns the HTTP response generated by the evidence submission process.
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
/// Asynchronously handles the attachment of dispute evidence by parsing the multipart request,
/// extracting the attach evidence request, and then passing the request to the appropriate
/// function for processing. It utilizes the server_wrap method to wrap the processing logic
/// in a server context, applying authentication and locking as necessary.
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
/// Diputes - Retrieve Dispute
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
/// Retrieve the evidence for a dispute by sending a request to the server and returning the response.
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
