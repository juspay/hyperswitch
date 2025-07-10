use api_models::chat as chat_api;
use common_utils::{
    errors::CustomResult,
    request::{Method, RequestBuilder, RequestContent},
};
use error_stack::ResultExt;
use external_services::http_client;
use hyperswitch_domain_models::chat as chat_domain;

use crate::{
    db::errors::chat::ChatErrors,
    routes::SessionState,
    services::{authentication as auth, ApplicationResponse},
};

pub async fn get_data_from_automation_workflow(
    state: SessionState,
    user_from_token: auth::UserFromToken,
    req: chat_api::ChatRequest,
) -> CustomResult<ApplicationResponse<chat_api::AutomationAiDataResponse>, ChatErrors> {
    let url = format!(
        "{}/webhook/n8n?message={}",
        state.conf.chat.automation_workflow_host, req.message
    );

    let request_body = chat_domain::AutomationAiDataRequest {
        org_id: user_from_token.org_id,
        merchant_id: user_from_token.merchant_id,
        profile_id: user_from_token.profile_id,
    };

    let request = RequestBuilder::new()
        .method(Method::Post)
        .url(&url)
        .attach_default_headers()
        .set_body(RequestContent::Json(Box::new(request_body)))
        .build();

    let response = http_client::send_request(&state.conf.proxy, request, None)
        .await
        .change_context(ChatErrors::InternalServerError)
        .attach_printable("Error in send request")?
        .json::<_>()
        .await
        .change_context(ChatErrors::ChatResponseDeserializationFailed)
        .attach_printable("Error in deserializing response")?;
    Ok(ApplicationResponse::Json(response))
}

pub async fn get_data_from_embedded_workflow(
    state: SessionState,
    user_from_token: auth::UserFromToken,
    req: chat_api::ChatRequest,
) -> CustomResult<ApplicationResponse<chat_api::EmbeddedAiDataResponse>, ChatErrors> {
    let url = format!("{}/webhook", state.conf.chat.embedded_workflow_host);

    let request_body = chat_domain::EmbeddedAiDataRequest {
        query: chat_domain::GetDataMessage {
            message: req.message,
        },
        org_id: user_from_token.org_id,
        merchant_id: user_from_token.merchant_id,
        profile_id: user_from_token.profile_id,
    };

    let request = RequestBuilder::new()
        .method(Method::Post)
        .url(&url)
        .attach_default_headers()
        .set_body(RequestContent::Json(Box::new(request_body)))
        .build();

    let response = http_client::send_request(&state.conf.proxy, request, None)
        .await
        .change_context(ChatErrors::InternalServerError)
        .attach_printable("Error in send request")?
        .json::<_>()
        .await
        .change_context(ChatErrors::InternalServerError)
        .attach_printable("Error in deserializing response")?;

    Ok(ApplicationResponse::Json(response))
}
