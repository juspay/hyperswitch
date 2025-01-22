use actix_web::{web, HttpRequest, HttpResponse};
use api_models::card_testing_guard as api_card_testing_guard;
use router_env::Flow;

use crate::{
    core::{api_locking, card_testing_guard},
    routes::AppState,
    services::{api, authentication as auth, authorization::permissions::Permission},
};

#[utoipa::path(
    post,
    path = "/cardtestingguard",
    request_body = api_card_testing_guard::UpdateCardTestingGuardRequest,
    responses(
        (status = 200, description = "Card Testing Guard Ststus", body = api_card_testing_guard::UpdateCardTestingGuardResponse),
    ),
    tag = "Card Testing Guard",
    operation_id = "Update Card Testing Guard STatus",
    security(("api_key" = []))
)]
pub async fn update_card_testing_guard(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<api_card_testing_guard::UpdateCardTestingGuardRequest>,
) -> HttpResponse {
    let flow = Flow::ListBlocklist;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, payload, _| {
            card_testing_guard::update_card_testing_guard(state, auth.merchant_account, payload)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::MerchantAccountWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}