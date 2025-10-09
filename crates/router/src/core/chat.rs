use api_models::chat as chat_api;
use common_utils::{
    consts,
    crypto::{DecodeMessage, GcmAes256},
    errors::CustomResult,
    request::{Method, RequestBuilder, RequestContent},
};
use error_stack::ResultExt;
use external_services::http_client;
use hyperswitch_domain_models::chat as chat_domain;
use masking::ExposeInterface;
use router_env::{
    instrument, logger,
    tracing::{self, Instrument},
};

use crate::{
    db::errors::chat::ChatErrors,
    routes::{app::SessionStateInfo, SessionState},
    services::{authentication as auth, authorization::roles, ApplicationResponse},
    utils,
};

#[instrument(skip_all, fields(?session_id))]
pub async fn get_data_from_hyperswitch_ai_workflow(
    state: SessionState,
    user_from_token: auth::UserFromToken,
    req: chat_api::ChatRequest,
    session_id: Option<&str>,
) -> CustomResult<ApplicationResponse<chat_api::ChatResponse>, ChatErrors> {
    let role_info = roles::RoleInfo::from_role_id_org_id_tenant_id(
        &state,
        &user_from_token.role_id,
        &user_from_token.org_id,
        user_from_token
            .tenant_id
            .as_ref()
            .unwrap_or(&state.tenant.tenant_id),
    )
    .await
    .change_context(ChatErrors::InternalServerError)
    .attach_printable("Failed to retrieve role information")?;
    let url = format!(
        "{}/webhook",
        state.conf.chat.get_inner().hyperswitch_ai_host
    );
    let request_id = state
        .get_request_id()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let request_body = chat_domain::HyperswitchAiDataRequest {
        query: chat_domain::GetDataMessage {
            message: req.message.clone(),
        },
        org_id: user_from_token.org_id.clone(),
        merchant_id: user_from_token.merchant_id.clone(),
        profile_id: user_from_token.profile_id.clone(),
        entity_type: role_info.get_entity_type(),
    };
    logger::info!("Request for AI service: {:?}", request_body);

    let mut request_builder = RequestBuilder::new()
        .method(Method::Post)
        .url(&url)
        .attach_default_headers()
        .header(consts::X_REQUEST_ID, &request_id)
        .set_body(RequestContent::Json(Box::new(request_body.clone())));

    if let Some(session_id) = session_id {
        request_builder = request_builder.header(consts::X_CHAT_SESSION_ID, session_id);
    }

    let request = request_builder.build();

    let response = http_client::send_request(
        &state.conf.proxy,
        request,
        Some(consts::REQUEST_TIME_OUT_FOR_AI_SERVICE),
    )
    .await
    .change_context(ChatErrors::InternalServerError)
    .attach_printable("Error when sending request to AI service")?
    .json::<chat_api::ChatResponse>()
    .await
    .change_context(ChatErrors::InternalServerError)
    .attach_printable("Error when deserializing response from AI service")?;

    let response_to_return = response.clone();
    tokio::spawn(
        async move {
            let new_hyperswitch_ai_interaction = utils::chat::construct_hyperswitch_ai_interaction(
                &state,
                &user_from_token,
                &req,
                &response,
                &request_id,
            )
            .await;

            match new_hyperswitch_ai_interaction {
                Ok(interaction) => {
                    let db = state.store.as_ref();
                    if let Err(e) = db.insert_hyperswitch_ai_interaction(interaction).await {
                        logger::error!("Failed to insert hyperswitch_ai_interaction: {:?}", e);
                    }
                }
                Err(e) => {
                    logger::error!("Failed to construct hyperswitch_ai_interaction: {:?}", e);
                }
            }
        }
        .in_current_span(),
    );

    Ok(ApplicationResponse::Json(response_to_return))
}

#[instrument(skip_all)]
pub async fn list_chat_conversations(
    state: SessionState,
    user_from_token: auth::UserFromToken,
    req: chat_api::ChatListRequest,
) -> CustomResult<ApplicationResponse<chat_api::ChatListResponse>, ChatErrors> {
    let role_info = roles::RoleInfo::from_role_id_org_id_tenant_id(
        &state,
        &user_from_token.role_id,
        &user_from_token.org_id,
        user_from_token
            .tenant_id
            .as_ref()
            .unwrap_or(&state.tenant.tenant_id),
    )
    .await
    .change_context(ChatErrors::InternalServerError)
    .attach_printable("Failed to retrieve role information")?;

    if !role_info.is_internal() {
        return Err(error_stack::Report::new(ChatErrors::UnauthorizedAccess)
            .attach_printable("Only internal roles are allowed for this operation"));
    }

    let db = state.store.as_ref();
    let hyperswitch_ai_interactions = db
        .list_hyperswitch_ai_interactions(
            req.merchant_id,
            req.limit.unwrap_or(consts::DEFAULT_LIST_LIMIT),
            req.offset.unwrap_or(consts::DEFAULT_LIST_OFFSET),
        )
        .await
        .change_context(ChatErrors::InternalServerError)
        .attach_printable("Error when fetching hyperswitch_ai_interactions")?;

    let encryption_key = state.conf.chat.get_inner().encryption_key.clone().expose();
    let key = match hex::decode(&encryption_key) {
        Ok(key) => key,
        Err(e) => {
            router_env::logger::error!("Failed to decode encryption key: {}", e);
            encryption_key.as_bytes().to_vec()
        }
    };

    let mut conversations = Vec::new();

    for interaction in hyperswitch_ai_interactions {
        let user_query_encrypted = interaction
            .user_query
            .ok_or(ChatErrors::InternalServerError)
            .attach_printable("Missing user_query field in hyperswitch_ai_interaction")?;
        let response_encrypted = interaction
            .response
            .ok_or(ChatErrors::InternalServerError)
            .attach_printable("Missing response field in hyperswitch_ai_interaction")?;

        let user_query_decrypted_bytes = GcmAes256
            .decode_message(&key, user_query_encrypted.into_inner())
            .change_context(ChatErrors::InternalServerError)
            .attach_printable("Failed to decrypt user query")?;

        let response_decrypted_bytes = GcmAes256
            .decode_message(&key, response_encrypted.into_inner())
            .change_context(ChatErrors::InternalServerError)
            .attach_printable("Failed to decrypt response")?;

        let user_query_decrypted = String::from_utf8(user_query_decrypted_bytes)
            .change_context(ChatErrors::InternalServerError)
            .attach_printable("Failed to convert decrypted user query to string")?;

        let response_decrypted = serde_json::from_slice(&response_decrypted_bytes)
            .change_context(ChatErrors::InternalServerError)
            .attach_printable("Failed to deserialize decrypted response")?;

        conversations.push(chat_api::ChatConversation {
            id: interaction.id,
            session_id: interaction.session_id,
            user_id: interaction.user_id,
            merchant_id: interaction.merchant_id,
            profile_id: interaction.profile_id,
            org_id: interaction.org_id,
            role_id: interaction.role_id,
            user_query: user_query_decrypted.into(),
            response: response_decrypted,
            database_query: interaction.database_query,
            interaction_status: interaction.interaction_status,
            created_at: interaction.created_at,
        });
    }

    return Ok(ApplicationResponse::Json(chat_api::ChatListResponse {
        conversations,
    }));
}
