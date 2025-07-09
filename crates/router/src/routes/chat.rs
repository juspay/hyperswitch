use actix_web::{web, HttpRequest, HttpResponse};
#[cfg(all(feature = "olap"))]
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
    query: web::Query<chat::ChatMessageQueryParam>,
) -> HttpResponse {
    let flow = Flow::GetDataFromAutomationFlow;
    let query_params = query.into_inner();
    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        (),
        |state, _: (), _, _| {
            chat_core::get_data_from_automation_workflow(state, query_params.clone())
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
    query: web::Query<chat::ChatMessageQueryParam>,
) -> HttpResponse {
    let flow = Flow::GetDataFromEmbeddedFlow;
    let query_params = query.into_inner();
    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        (),
        |state, _: (), _, _| {
            chat_core::get_data_from_embedded_workflow(state, query_params.clone())
        },
        &auth::JWTAuth {
            permission: Permission::MerchantPaymentRead,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
