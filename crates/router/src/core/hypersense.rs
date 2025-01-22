use api_models::hypersense as hypersense_api;
use error_stack::ResultExt;
use masking::ExposeInterface;

use crate::{
    core::errors::{self, RouterResponse},
    db::domain::UserFromStorage,
    services::{
        api as service_api,
        authentication::{self, ExternalServiceType, ExternalToken},
    },
    SessionState,
};

pub async fn generate_hypersense_token(
    state: SessionState,
    user: authentication::UserFromToken,
) -> RouterResponse<hypersense_api::HypersenseTokenResponse> {
    let token = ExternalToken::new_token(
        user.user_id.clone(),
        user.merchant_id.clone(),
        &state.conf,
        ExternalServiceType::Hypersense,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable_lazy(|| {
        format!(
            "Failed to create hypersense token for params [user_id, mid] [{}, {:?}]",
            user.user_id, user.merchant_id,
        )
    })?;

    Ok(service_api::ApplicationResponse::Json(
        hypersense_api::HypersenseTokenResponse {
            token: token.into(),
        },
    ))
}

pub async fn verify_hypersense_token(
    state: SessionState,
    json_payload: hypersense_api::HypersenseVerifyTokenRequest,
) -> RouterResponse<hypersense_api::HypersenseVerifyTokenResponse> {
    let token_from_payload = json_payload.token.expose();

    let token = authentication::decode_jwt::<ExternalToken>(&token_from_payload, &state)
        .await
        .change_context(errors::ApiErrorResponse::Unauthorized)?;

    token.check_service_type(&ExternalServiceType::Hypersense)?;

    let UserFromStorage(user_in_db) = token
        .get_user_from_db(&state)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!(
                "Failed to fetch the user from DB for user_id - {}",
                &token.user_id
            )
        })?;

    let email = user_in_db.email.clone();

    Ok(service_api::ApplicationResponse::Json(
        hypersense_api::HypersenseVerifyTokenResponse {
            user_id: user_in_db.user_id,
            merchant_id: token.merchant_id,
            email,
        },
    ))
}
