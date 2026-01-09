use actix_web::{web, HttpRequest, HttpResponse};
#[cfg(feature = "olap")]
use api_models::chat as chat_api;
use router_env::{instrument, tracing, Flow};

use super::AppState;
use crate::{
    core::{api_locking, chat as chat_core},
    routes::metrics,
    services::{
        api,
        authentication::{self as auth},
        authorization::permissions::Permission,
    },
};

#[instrument(skip_all)]
pub async fn get_data_from_hyperswitch_ai_workflow(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    payload: web::Json<chat_api::ChatRequest>,
) -> HttpResponse {
    let flow = Flow::GetDataFromHyperswitchAiFlow;
    let session_id = http_req
        .headers()
        .get(common_utils::consts::X_CHAT_SESSION_ID)
        .and_then(|header_value| header_value.to_str().ok());
    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        payload.into_inner(),
        |state, user: auth::UserFromToken, payload, _| {
            metrics::CHAT_REQUEST_COUNT.add(
                1,
                router_env::metric_attributes!(("merchant_id", user.merchant_id.clone())),
            );
            chat_core::get_data_from_hyperswitch_ai_workflow(state, user, payload, session_id)
        },
        // At present, the AI service retrieves data scoped to the merchant level
        &auth::JWTAuth {
            permission: Permission::MerchantPaymentRead,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all)]
pub async fn get_all_conversations(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    payload: web::Query<chat_api::ChatListRequest>,
) -> HttpResponse {
    let flow = Flow::ListAllChatInteractions;
    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        payload.into_inner(),
        |state, user: auth::UserFromToken, payload, _| {
            chat_core::list_chat_conversations(state, user, payload)
        },
        &auth::DashboardNoPermissionAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
