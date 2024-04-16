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
        |state, _, payload, _| gsm::create_gsm_rule(state, payload),
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
        |state, _, gsm_retrieve_req, _| gsm::retrieve_gsm_rule(state, gsm_retrieve_req),
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
        |state, _, payload, _| gsm::update_gsm_rule(state, payload),
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
        |state, _, payload, _| gsm::delete_gsm_rule(state, payload),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
