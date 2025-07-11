use actix_web::{web, HttpRequest, HttpResponse};
#[cfg(feature = "olap")]
use api_models::chat as chat_api;
use router_env::Flow;

use super::AppState;
use crate::{
    core::{api_locking, chat as chat_core},
    services::{
        api,
        authentication::{self as auth},
        authorization::permissions::Permission,
    },
};

pub async fn get_data_from_hyperswitch_ai_workflow(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    payload: web::Json<chat_api::ChatRequest>,
) -> HttpResponse {
    let flow = Flow::GetDataFromHyperswitchAiFlow;
    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        payload.into_inner(),
        |state, user: auth::UserFromToken, payload, _| {
            chat_core::get_data_from_hyperswitch_ai_workflow(state, user, payload)
        },
        &auth::JWTAuth {
            permission: Permission::MerchantPaymentRead,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
