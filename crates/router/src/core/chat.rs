use api_models::chat::{self, ChatDataType};
use common_utils::{
    errors::CustomResult,
    request::{Method, RequestBuilder},
};
use error_stack::ResultExt;
use external_services::http_client;

use crate::{
    db::errors::chat::ChatErrors,
    routes::SessionState,
    services::{authentication as auth, ApplicationResponse},
};

pub async fn ask_chat(
    state: SessionState,
    _user_from_token: auth::UserFromToken,
    message: String,
) -> CustomResult<ApplicationResponse<chat::ChatResponse>, ChatErrors> {
    let request = RequestBuilder::new()
        .method(Method::Get)
        .url("http://localhost:8080?message=${message}")
        .build();

    let response = http_client::send_request(&state.conf.proxy, request, None)
        .await
        .change_context(ChatErrors::InternalServerError)?
        .json::<ChatDataType>()
        .await
        .change_context(ChatErrors::ChatResponseDeserializationFailed)?;

    let res = response
        .output
        .unwrap_or_else(|| serde_json::json!("No data returned from external API"));

    Ok(ApplicationResponse::Json(chat::ChatResponse {
        message: message.clone(),
        data: res,
        timestamp: common_utils::date_time::now().to_string(),
    }))
}
