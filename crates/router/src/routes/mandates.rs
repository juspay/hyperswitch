use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::mandate,
    services::{api, authentication as auth},
    types::api::mandates,
};

/// Mandates - Retrieve Mandate
///
/// Retrieve a mandate
#[utoipa::path(
    get,
    path = "/mandates/{mandate_id}",
    params(
        ("mandate_id" = String, Path, description = "The identifier for mandate")
    ),
    responses(
        (status = 200, description = "The mandate was retrieved successfully", body = MandateResponse),
        (status = 404, description = "Mandate does not exist in our records")
    ),
    tag = "Mandates",
    operation_id = "Retrieve a Mandate",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::MandatesRetrieve))]
// #[get("/{id}")]
pub async fn get_mandate(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::MandatesRetrieve;
    let mandate_id = mandates::MandateId {
        mandate_id: path.into_inner(),
    };
    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        mandate_id,
        |state, auth, req| mandate::get_mandate(state, auth.merchant_account, req),
        &auth::ApiKeyAuth,
    )
    .await
}

/// Mandates - Revoke Mandate
///
/// Revoke a mandate
#[utoipa::path(
    post,
    path = "/mandates/revoke/{mandate_id}",
    params(
        ("mandate_id" = String, Path, description = "The identifier for mandate")
    ),
    responses(
        (status = 200, description = "The mandate was revoked successfully", body = MandateRevokedResponse),
        (status = 400, description = "Mandate does not exist in our records")
    ),
    tag = "Mandates",
    operation_id = "Revoke a Mandate",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::MandatesRevoke))]
// #[post("/revoke/{id}")]
pub async fn revoke_mandate(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::MandatesRevoke;
    let mandate_id = mandates::MandateId {
        mandate_id: path.into_inner(),
    };
    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        mandate_id,
        |state, auth, req| mandate::revoke_mandate(&*state.store, auth.merchant_account, req),
        &auth::ApiKeyAuth,
    )
    .await
}

/// Mandates - List Mandates
#[utoipa::path(
    get,
    path = "/mandates/list",
    params(
        ("limit" = Option<i64>, Query, description = "The maximum number of Mandate Objects to include in the response"),
        ("mandate_status" = Option<MandateStatus>, Query, description = "The status of mandate"),
        ("connector" = Option<String>, Query, description = "The connector linked to mandate"),
        ("created_time" = Option<PrimitiveDateTime>, Query, description = "The time at which mandate is created"),
        ("created_time.lt" = Option<PrimitiveDateTime>, Query, description = "Time less than the mandate created time"),
        ("created_time.gt" = Option<PrimitiveDateTime>, Query, description = "Time greater than the mandate created time"),
        ("created_time.lte" = Option<PrimitiveDateTime>, Query, description = "Time less than or equals to the mandate created time"),
        ("created_time.gte" = Option<PrimitiveDateTime>, Query, description = "Time greater than or equals to the mandate created time"),
    ),
    responses(
        (status = 200, description = "The mandate list was retrieved successfully", body = Vec<MandateResponse>),
        (status = 401, description = "Unauthorized request")
    ),
    tag = "Mandates",
    operation_id = "List Mandates",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::MandatesList))]
pub async fn retrieve_mandates_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Query<api_models::mandates::MandateListConstraints>,
) -> HttpResponse {
    let flow = Flow::MandatesList;
    let payload = payload.into_inner();
    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        payload,
        |state, auth, req| mandate::retrieve_mandates_list(state, auth.merchant_account, req),
        auth::auth_type(&auth::ApiKeyAuth, &auth::JWTAuth, req.headers()),
    )
    .await
}
