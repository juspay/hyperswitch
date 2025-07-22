use api_models::chat as chat_api;
use common_utils::{
    consts,
    errors::CustomResult,
    request::{Method, RequestBuilder, RequestContent},
};
use error_stack::ResultExt;
use external_services::http_client;
use hyperswitch_domain_models::chat as chat_domain;
use router_env::{instrument, logger, tracing};

use crate::{
    db::errors::chat::ChatErrors,
    routes::SessionState,
    services::{authentication as auth, ApplicationResponse},
};

#[instrument(skip_all)]
pub async fn get_data_from_hyperswitch_ai_workflow(
    state: SessionState,
    user_from_token: auth::UserFromToken,
    req: chat_api::ChatRequest,
) -> CustomResult<ApplicationResponse<chat_api::ChatResponse>, ChatErrors> {
    let url = format!("{}/webhook", state.conf.chat.hyperswitch_ai_host);

    let request_body = chat_domain::HyperswitchAiDataRequest {
        query: chat_domain::GetDataMessage {
            message: req.message,
        },
        org_id: user_from_token.org_id,
        merchant_id: user_from_token.merchant_id,
        profile_id: user_from_token.profile_id,
    };
    logger::info!("Request for AI service: {:?}", request_body);

    let request = RequestBuilder::new()
        .method(Method::Post)
        .url(&url)
        .attach_default_headers()
        .set_body(RequestContent::Json(Box::new(request_body.clone())))
        .build();

    let response = http_client::send_request(
        &state.conf.proxy,
        request,
        Some(consts::REQUEST_TIME_OUT_FOR_AI_SERVICE),
    )
    .await
    .change_context(ChatErrors::InternalServerError)
    .attach_printable("Error when sending request to AI service")?
    .json::<_>()
    .await
    .change_context(ChatErrors::InternalServerError)
    .attach_printable("Error when deserializing response from AI service")?;

    Ok(ApplicationResponse::Json(response))
}
