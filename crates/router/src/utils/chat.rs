use api_models::chat;
use common_utils::{
    errors::CustomResult,
    request::{Method, RequestBuilder},
};
use error_stack::ResultExt;
use external_services::http_client;

use crate::{db::errors::chat::ChatErrors, routes::SessionState, services::ApplicationResponse};

pub async fn make_chat_request_and_get_response(
    state: &SessionState,
    message: &str,
    url: &str,
) -> CustomResult<ApplicationResponse<chat::ChatResponse>, ChatErrors> {
    let formatted_url = format!("{}?message={}", url, message);
    let request = RequestBuilder::new()
        .method(Method::Get)
        .url(&formatted_url)
        .build();

    let response = http_client::send_request(&state.conf.proxy, request, None)
        .await
        .change_context(ChatErrors::InternalServerError)?
        .json::<chat::ChatDataType>()
        .await
        .change_context(ChatErrors::ChatResponseDeserializationFailed)?;

    let res = response
        .output
        .unwrap_or_else(|| serde_json::json!("No data returned from external API"));

    Ok(ApplicationResponse::Json(chat::ChatResponse {
        message: message.to_string(),
        data: res,
        timestamp: common_utils::date_time::now().to_string(),
    }))
}
