use actix_web::{web, HttpRequest, HttpResponse};
#[cfg(feature = "olap")]
use api_models::chat;
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

pub async fn get_data_from_automation_workflow(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    payload: web::Json<chat::AutomationAiGetDataRequest>,
    query: web::Query<chat::GetDataMessage>,
) -> HttpResponse {
    let flow = Flow::GetDataFromAutomationFlow;
    let query = query.into_inner();
    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        payload.into_inner(),
        |state, _: (), payload, _| {
            chat_core::get_data_from_automation_workflow(state, payload, query.clone())
        },
        &auth::JWTAuth {
            permission: Permission::MerchantPaymentRead,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn get_data_from_embedded_workflow(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    payload: web::Json<chat::EmbeddedAiGetDataRequest>,
) -> HttpResponse {
    let flow = Flow::GetDataFromEmbeddedFlow;
    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        payload.into_inner(),
        |state, _: (), payload, _| chat_core::get_data_from_embedded_workflow(state, payload),
        &auth::JWTAuth {
            permission: Permission::MerchantPaymentRead,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
