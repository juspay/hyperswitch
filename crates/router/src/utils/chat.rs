use api_models::chat as chat_api;
use common_utils::{
    crypto::{EncodeMessage, GcmAes256},
    encryption::Encryption,
};
use diesel_models::hyperswitch_ai_interaction::HyperswitchAiInteractionNew;
use error_stack::ResultExt;
use masking::ExposeInterface;

use crate::{
    core::errors::{self, CustomResult},
    routes::SessionState,
    services::authentication as auth,
};

pub async fn construct_hyperswitch_ai_interaction(
    state: &SessionState,
    user_from_token: &auth::UserFromToken,
    req: &chat_api::ChatRequest,
    response: &chat_api::ChatResponse,
    request_id: &str,
) -> CustomResult<HyperswitchAiInteractionNew, errors::ApiErrorResponse> {
    let encryption_key = state.conf.chat.get_inner().encryption_key.clone().expose();
    let key = match hex::decode(&encryption_key) {
        Ok(key) => key,
        Err(e) => {
            router_env::logger::error!("Failed to decode encryption key: {}", e);
            // Fallback to using the string as bytes, which was the previous behavior
            encryption_key.as_bytes().to_vec()
        }
    };
    let encrypted_user_query_bytes = GcmAes256
        .encode_message(&key, &req.message.clone().expose().into_bytes())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to encrypt user query")?;

    let encrypted_response_bytes = serde_json::to_vec(&response.response.clone())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialize response for encryption")
        .and_then(|bytes| {
            GcmAes256
                .encode_message(&key, &bytes)
                .change_context(errors::ApiErrorResponse::InternalServerError)
        })
        .attach_printable("Failed to encrypt response")?;

    Ok(HyperswitchAiInteractionNew {
        id: request_id.to_owned(),
        session_id: Some(request_id.to_string()),
        user_id: Some(user_from_token.user_id.clone()),
        merchant_id: Some(user_from_token.merchant_id.get_string_repr().to_string()),
        profile_id: Some(user_from_token.profile_id.get_string_repr().to_string()),
        org_id: Some(user_from_token.org_id.get_string_repr().to_string()),
        role_id: Some(user_from_token.role_id.clone()),
        user_query: Some(Encryption::new(encrypted_user_query_bytes.into())),
        response: Some(Encryption::new(encrypted_response_bytes.into())),
        database_query: response.query_executed.clone().map(|q| q.expose()),
        interaction_status: Some(response.status.clone()),
        created_at: common_utils::date_time::now(),
    })
}
