use api_models::external_service_auth as external_service_auth_api;
use common_utils::fp_utils;
use error_stack::ResultExt;
use masking::ExposeInterface;

use crate::{
    core::errors::{self, RouterResponse},
    services::{
        api as service_api,
        authentication::{self, ExternalServiceType, ExternalToken},
    },
    SessionState,
};

pub async fn generate_external_token(
    state: SessionState,
    user: authentication::UserFromToken,
    external_service_type: ExternalServiceType,
) -> RouterResponse<external_service_auth_api::ExternalTokenResponse> {
    let token = ExternalToken::new_token(
        user.user_id.clone(),
        user.merchant_id.clone(),
        &state.conf,
        external_service_type.clone(),
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable_lazy(|| {
        format!(
            "Failed to create external token for params [user_id, mid, external_service_type] [{}, {:?}, {:?}]",
            user.user_id, user.merchant_id, external_service_type,
        )
    })?;

    Ok(service_api::ApplicationResponse::Json(
        external_service_auth_api::ExternalTokenResponse {
            token: token.into(),
        },
    ))
}

pub async fn signout_external_token(
    state: SessionState,
    json_payload: external_service_auth_api::ExternalSignoutTokenRequest,
) -> RouterResponse<()> {
    let token = authentication::decode_jwt::<ExternalToken>(&json_payload.token.expose(), &state)
        .await
        .change_context(errors::ApiErrorResponse::Unauthorized)?;

    authentication::blacklist::insert_user_in_blacklist(&state, &token.user_id)
        .await
        .change_context(errors::ApiErrorResponse::InvalidJwtToken)?;

    Ok(service_api::ApplicationResponse::StatusOk)
}

pub async fn verify_external_token(
    state: SessionState,
    json_payload: external_service_auth_api::ExternalVerifyTokenRequest,
    external_service_type: ExternalServiceType,
) -> RouterResponse<external_service_auth_api::ExternalVerifyTokenResponse> {
    let token_from_payload = json_payload.token.expose();

    let token = authentication::decode_jwt::<ExternalToken>(&token_from_payload, &state)
        .await
        .change_context(errors::ApiErrorResponse::Unauthorized)?;

    fp_utils::when(
        authentication::blacklist::check_user_in_blacklist(&state, &token.user_id, token.exp)
            .await?,
        || Err(errors::ApiErrorResponse::InvalidJwtToken),
    )?;

    token.check_service_type(&external_service_type)?;

    let user_in_db = state
        .global_store
        .find_user_by_id(&token.user_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("User not found in database")?;

    let email = user_in_db.email.clone();
    let name = user_in_db.name;

    Ok(service_api::ApplicationResponse::Json(
        external_service_auth_api::ExternalVerifyTokenResponse::Hypersense {
            user_id: user_in_db.user_id,
            merchant_id: token.merchant_id,
            name,
            email,
        },
    ))
}
