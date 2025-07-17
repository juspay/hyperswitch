use actix_web::{web, HttpRequest, HttpResponse};
#[cfg(feature = "olap")]
use api_models::chat as chat_api;
use router_env::{instrument, tracing, Flow};

use super::AppState;
use crate::{
    core::{api_locking, chat as chat_core},
    services::{
        api,
        authentication::{self as auth},
        authorization::permissions::Permission,
    },
    utils,
};

#[instrument(skip_all)]
pub async fn get_data_from_hyperswitch_ai_workflow(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    payload: web::Json<chat_api::ChatRequest>,
) -> HttpResponse {
    let flow = Flow::GetDataFromHyperswitchAiFlow;
    let request_id =
        utils::get_request_id(&http_req).unwrap_or_else(|_| uuid::Uuid::new_v4().to_string());
    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        payload.into_inner(),
        |state, user: auth::UserFromToken, payload, _| {
            chat_core::get_data_from_hyperswitch_ai_workflow(
                state,
                user,
                payload,
                request_id.clone(),
            )
        },
        // At present, the AI service retrieves data scoped to the merchant level
        &auth::JWTAuth {
            permission: Permission::MerchantPaymentRead,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
