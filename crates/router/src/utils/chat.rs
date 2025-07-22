use api_models::chat as chat_api;
use diesel_models::hyperswitch_ai_interaction::HyperswitchAiInteractionNew;
use masking::ExposeInterface;

use crate::services::authentication as auth;

pub fn construct_hyperswitch_ai_interaction(
    user_from_token: &auth::UserFromToken,
    req: &chat_api::ChatRequest,
    response: &chat_api::ChatResponse,
    request_id: &str,
) -> HyperswitchAiInteractionNew {
    HyperswitchAiInteractionNew {
        id: request_id.to_owned(),
        session_id: Some(request_id.to_string()),
        user_id: Some(user_from_token.user_id.clone()),
        merchant_id: Some(user_from_token.merchant_id.get_string_repr().to_string()),
        profile_id: Some(user_from_token.profile_id.get_string_repr().to_string()),
        org_id: Some(user_from_token.org_id.get_string_repr().to_string()),
        role_id: Some(user_from_token.role_id.clone()),
        user_query: Some(req.message.clone().expose()),
        response: Some(response.response.clone().expose().to_string()),
        database_query: response.query_executed.clone().map(|q| q.expose()),
        interaction_status: Some(response.status.clone()),
        created_at: common_utils::date_time::now(),
    }
}
