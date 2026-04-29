use actix_web::{web, HttpRequest, HttpResponse};
use api_models::card_issuer as api_types;
use common_utils::id_type;
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{api_locking, card_issuer},
    services::{api, authentication as auth, authorization::permissions::Permission},
};

#[instrument(skip_all, fields(flow = ?Flow::AddCardIssuer))]
pub async fn add_card_issuer(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<api_types::CardIssuerRequest>,
) -> HttpResponse {
    let flow = Flow::AddCardIssuer;
    let payload = json_payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, _, body, _| card_issuer::add_card_issuer(state, body),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::UpdateCardIssuer))]
pub async fn update_card_issuer(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::CardIssuerId>,
    json_payload: web::Json<api_types::CardIssuerUpdateRequest>,
) -> HttpResponse {
    let flow = Flow::UpdateCardIssuer;
    let id = path.into_inner();
    let payload = json_payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        move |state, _, body, _| card_issuer::update_card_issuer(state, id.clone(), body),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::ListCardIssuers))]
pub async fn list_card_issuers(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<api_types::CardIssuerListQuery>,
) -> HttpResponse {
    let flow = Flow::ListCardIssuers;
    let query = query.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        query,
        |state, _, query, _| card_issuer::list_card_issuers(state, query),
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                allow_connected_scope_operation: false,
                allow_platform_self_operation: false,
            }),
            &auth::JWTAuth {
                permission: Permission::MerchantAccountRead,
                allow_connected: false,
                allow_platform: false,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
