use api_models::chat;
use common_utils::{
    errors::CustomResult,
    request::{Method, RequestBuilder, RequestContent},
};
use error_stack::ResultExt;
use external_services::http_client;

use crate::{
    db::errors::chat::ChatErrors,
    routes::SessionState,
    services::{authentication as auth, ApplicationResponse},
};

pub async fn get_data_from_automation_workflow(
    state: SessionState,
    user_from_token: auth::UserFromToken,
    payload: chat::AutomationAiGetDataRequest,
    query: chat::GetDataMessage,
) -> CustomResult<ApplicationResponse<chat::AutomationAiDataResponse>, ChatErrors> {
    let url = format!(
        "{}/webhook/n8n?message={}",
        state.conf.chat.automation_workflow_host, query.message
    );
    let request = RequestBuilder::new()
        .method(Method::Post)
        .url(&url)
        .attach_default_headers()
        .set_body(RequestContent::Json(Box::new(payload)))
        .build();

    let response = http_client::send_request(&state.conf.proxy, request, None)
        .await
        .change_context(ChatErrors::InternalServerError)
        .attach_printable("Error in send request")?
        .json::<chat::AutomationAiDataResponse>()
        .await
        .change_context(ChatErrors::ChatResponseDeserializationFailed)
        .attach_printable("Error in deserializing response")?;
    Ok(ApplicationResponse::Json(response))
}

pub async fn get_data_from_embedded_workflow(
    state: SessionState,
    user_from_token: auth::UserFromToken,
    payload: chat::EmbeddedAiGetDataRequest,
) -> CustomResult<ApplicationResponse<chat::EmbeddedAiDataResponse>, ChatErrors> {
    let url = format!("{}/webhook", state.conf.chat.embedded_workflow_host);

    let request = RequestBuilder::new()
        .method(Method::Post)
        .url(&url)
        .attach_default_headers()
        .set_body(RequestContent::Json(Box::new(payload)))
        .build();

    let response = http_client::send_request(&state.conf.proxy, request, None)
        .await
        .change_context(ChatErrors::InternalServerError)
        .attach_printable("Error in send request")?
        .json::<chat::EmbeddedAiDataResponse>()
        .await
        .change_context(ChatErrors::InternalServerError)
        .attach_printable("Error in deserializing response")?;

    Ok(ApplicationResponse::Json(response))
}
