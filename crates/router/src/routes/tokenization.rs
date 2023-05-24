use actix_web::{web, HttpRequest, Responder};
use api_models::tokenization;
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::tokenization::*,
    services::{api, authentication as auth},
};

/// GetTrid
///
/// Onboard a merchant for tokenization with respective card network
#[utoipa::path(
    post,
    path = "/tokenization/getTrid",
    responses(
        (status = 200, description = "GetTrid: Tokenization onboarding", body = GetTrid),
        (status = 404, description = "GetTrid Failed")
    ),
    security(("api_key" = []), ("publishable_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::TokenizationGetTrid))]
pub async fn get_trid(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<tokenization::GetTrid>,
) -> impl Responder {
    let flow = Flow::TokenizationGetTrid;
    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        json_payload.into_inner(),
        get_trid_core,
        &auth::ApiKeyAuth,
    )
    .await
}
