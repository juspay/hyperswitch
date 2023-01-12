use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::mandate,
    services::{api, authentication as auth},
    types::api::mandates,
};

#[instrument(skip_all, fields(flow = ?Flow::MandatesRetrieve))]
// #[get("/{id}")]
pub async fn get_mandate(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let mandate_id = mandates::MandateId {
        mandate_id: path.into_inner(),
    };
    api::server_wrap(
        &state,
        &req,
        mandate_id,
        mandate::get_mandate,
        &auth::ApiKeyAuth,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::MandatesRevoke))]
// #[post("/revoke/{id}")]
pub async fn revoke_mandate(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let mandate_id = mandates::MandateId {
        mandate_id: path.into_inner(),
    };
    api::server_wrap(
        &state,
        &req,
        mandate_id,
        |state, merchant_account, req| {
            mandate::revoke_mandate(&*state.store, merchant_account, req)
        },
        &auth::ApiKeyAuth,
    )
    .await
}
