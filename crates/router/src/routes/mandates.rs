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
        mandate::get_mandate,
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
        |state, merchant_account, req| {
            mandate::revoke_mandate(&*state.store, merchant_account, req)
        },
        &auth::ApiKeyAuth,
    )
    .await
}
