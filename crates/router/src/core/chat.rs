use api_models::chat as chat_api;
use common_utils::{
    consts,
    errors::CustomResult,
    request::{Method, RequestBuilder, RequestContent},
};
use error_stack::ResultExt;
use external_services::http_client;
use hyperswitch_domain_models::chat as chat_domain;
use router_env::logger;

use crate::{
    db::errors::chat::ChatErrors,
    routes::SessionState,
    services::{authentication as auth, ApplicationResponse},
};

pub async fn get_data_from_hyperswitch_ai_workflow(
    state: SessionState,
    user_from_token: auth::UserFromToken,
    req: chat_api::ChatRequest,
) -> CustomResult<ApplicationResponse<chat_api::ChatResponse>, ChatErrors> {
    logger::debug!("Getting data from hyperswitch ai workflow");
    let url = format!("{}/webhook", state.conf.chat.hyperswitch_ai_host);
    logger::info!("Hyperswitch AI URL: {}", url);

    let request_body = chat_domain::EmbeddedAiDataRequest {
        query: chat_domain::GetDataMessage {
            message: req.message,
        },
        org_id: user_from_token.org_id,
        merchant_id: user_from_token.merchant_id,
        profile_id: user_from_token.profile_id,
    };
    logger::info!("Request body for AI service: {:?}", request_body);

    let request = RequestBuilder::new()
        .method(Method::Post)
        .url(&url)
        .attach_default_headers()
        .set_body(RequestContent::Json(Box::new(request_body)))
        .build();
    logger::info!("Request: {:?}", request);

    let response = http_client::send_request(
        &state.conf.proxy,
        request,
        Some(consts::REQUEST_TIME_OUT_FOR_AI_SERVICE),
    )
    .await
    .change_context(ChatErrors::InternalServerError)
        .attach_printable("Error in send request")?
        .json::<chat_api::ChatResponse>()
        .await
        .change_context(ChatErrors::InternalServerError)
        .attach_printable("Error in deserializing response")?;

    logger::info!("Response from AI service: {:?}", response);
    Ok(ApplicationResponse::Json(response))
}
