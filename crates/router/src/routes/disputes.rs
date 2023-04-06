use actix_web::{web, HttpRequest, HttpResponse};
use api_models::disputes::DisputeListConstraints;
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::disputes,
    services::{api, authentication as auth},
    types::api::disputes as dispute_types,
};

/// Diputes - Retrieve Dispute
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
        state.get_ref(),
        &req,
        dispute_id,
        disputes::retrieve_dispute,
        auth::auth_type(&auth::ApiKeyAuth, &auth::JWTAuth, req.headers()),
    )
    .await
}

/// Diputes - List Disputes
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
    payload: web::Query<DisputeListConstraints>,
) -> HttpResponse {
    let flow = Flow::DisputesList;
    let payload = payload.into_inner();
    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        payload,
        disputes::retrieve_disputes_list,
        auth::auth_type(&auth::ApiKeyAuth, &auth::JWTAuth, req.headers()),
    )
    .await
}
