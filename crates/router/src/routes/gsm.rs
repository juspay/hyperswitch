use actix_web::{web, HttpRequest, Responder};
use api_models::gsm as gsm_api_types;
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{api_locking, gsm},
    services::{api, authentication as auth},
};

/// Gsm - Create
///
/// To create a Gsm Rule
#[utoipa::path(
    post,
    path = "/gsm",
    request_body(
        content = GsmCreateRequest,
    ),
    responses(
        (status = 200, description = "Gsm created", body = GsmResponse),
        (status = 400, description = "Missing Mandatory fields")
    ),
    tag = "Gsm",
    operation_id = "Create Gsm Rule",
    security(("admin_api_key" = [])),
)]
#[instrument(skip_all, fields(flow = ?Flow::GsmRuleCreate))]
/// Asynchronously creates a GSM rule using the provided JSON payload. This method
/// takes the application state, HTTP request, and JSON payload as input, and then
/// passes them to the `api::server_wrap` function along with the necessary flow,
/// authentication, and locking parameters to create the GSM rule using the
/// `gsm::create_gsm_rule` function. The method returns a `Responder` which will be
/// awaited to produce the final response.
pub async fn create_gsm_rule(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<gsm_api_types::GsmCreateRequest>,
) -> impl Responder {
    let payload = json_payload.into_inner();

    let flow = Flow::GsmRuleCreate;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        payload,
        |state, _, payload| gsm::create_gsm_rule(state, payload),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Gsm - Get
///
/// To get a Gsm Rule
#[utoipa::path(
    post,
    path = "/gsm/get",
    request_body(
        content = GsmRetrieveRequest,
    ),
    responses(
        (status = 200, description = "Gsm retrieved", body = GsmResponse),
        (status = 400, description = "Missing Mandatory fields")
    ),
    tag = "Gsm",
    operation_id = "Retrieve Gsm Rule",
    security(("admin_api_key" = [])),
)]
#[instrument(skip_all, fields(flow = ?Flow::GsmRuleRetrieve))]
/// Retrieves a GSM rule using the provided JSON payload and returns a responder.
pub async fn get_gsm_rule(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<gsm_api_types::GsmRetrieveRequest>,
) -> impl Responder {
    let gsm_retrieve_req = json_payload.into_inner();
    let flow = Flow::GsmRuleRetrieve;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        gsm_retrieve_req,
        |state, _, gsm_retrieve_req| gsm::retrieve_gsm_rule(state, gsm_retrieve_req),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Gsm - Update
///
/// To update a Gsm Rule
#[utoipa::path(
    post,
    path = "/gsm/update",
    request_body(
        content = GsmUpdateRequest,
    ),
    responses(
        (status = 200, description = "Gsm updated", body = GsmResponse),
        (status = 400, description = "Missing Mandatory fields")
    ),
    tag = "Gsm",
    operation_id = "Update Gsm Rule",
    security(("admin_api_key" = [])),
)]
#[instrument(skip_all, fields(flow = ?Flow::GsmRuleUpdate))]
/// Asynchronously updates a GSM rule using the provided JSON payload and returns the updated rule.
pub async fn update_gsm_rule(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<gsm_api_types::GsmUpdateRequest>,
) -> impl Responder {
    let payload = json_payload.into_inner();

    let flow = Flow::GsmRuleUpdate;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        payload,
        |state, _, payload| gsm::update_gsm_rule(state, payload),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Gsm - Delete
///
/// To delete a Gsm Rule
#[utoipa::path(
    post,
    path = "/gsm/delete",
    request_body(
        content = GsmDeleteRequest,
    ),
    responses(
        (status = 200, description = "Gsm deleted", body = GsmDeleteResponse),
        (status = 400, description = "Missing Mandatory fields")
    ),
    tag = "Gsm",
    operation_id = "Delete Gsm Rule",
    security(("admin_api_key" = [])),
)]
#[instrument(skip_all, fields(flow = ?Flow::GsmRuleDelete))]
/// Asynchronously handles the deletion of a GSM rule by wrapping the delete_gsm_rule function in a server_wrap call.
pub async fn delete_gsm_rule(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<gsm_api_types::GsmDeleteRequest>,
) -> impl Responder {
    let payload = json_payload.into_inner();

    let flow = Flow::GsmRuleDelete;

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, _, payload| gsm::delete_gsm_rule(state, payload),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
