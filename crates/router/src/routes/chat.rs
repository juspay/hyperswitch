use super::AppState;
use crate::{
    core::{api_locking, chat as chat_core},
    services::{
        api,
        authentication::{self as auth},
        authorization::permissions::Permission,
    },
};
use actix_web::{web, HttpRequest, HttpResponse};
use api_models::chat::{self as chat_api};
use router_env::Flow;

pub async fn ask_chat(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    query: web::Query<chat_api::ChatMessageQueryParam>,
) -> HttpResponse {
    let flow = Flow::UserSignUp;
    let query_params = query.into_inner();
    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        (),
        |state, user, _, _| chat_core::ask_chat(state, user, query_params.message.clone()),
        &auth::JWTAuth {
            permission: Permission::MerchantPaymentRead,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
