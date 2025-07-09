use api_models::chat;
use common_utils::errors::CustomResult;
use crate::{
    db::errors::chat::ChatErrors, routes::SessionState, services::ApplicationResponse, utils,
};

pub async fn get_data_from_automation_workflow(
    state: SessionState,
    query: chat::ChatMessageQueryParam,
) -> CustomResult<ApplicationResponse<chat::ChatResponse>, ChatErrors> {
    utils::chat::make_chat_request_and_get_response(
        &state,
        &query.message,
        &state.conf.chat.automation_workflow_host,
    )
    .await
}

pub async fn get_data_from_embedded_workflow(
    state: SessionState,
    query: chat::ChatMessageQueryParam,
) -> CustomResult<ApplicationResponse<chat::ChatResponse>, ChatErrors> {
    utils::chat::make_chat_request_and_get_response(
        &state,
        &query.message,
        &state.conf.chat.embedded_workflow_host,
    )
    .await
}
