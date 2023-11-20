use api_models::user as api;
use diesel_models::enums::UserStatus;
use masking::{ExposeInterface, Secret};

use super::errors::{UserErrors, UserResponse};
use crate::{
    consts::user as consts, routes::AppState, services::ApplicationResponse, types::domain,
};

pub async fn signup(
    state: AppState,
    request: api::SignUpRequest,
) -> UserResponse<api::SignUpResponse> {
    let new_user = domain::NewUser::try_from(request)?;
    let _ = new_user
        .get_new_merchant()
        .get_new_organization()
        .insert_org_in_db(state.clone())
        .await?;
    let user_from_db = new_user
        .insert_user_and_merchant_in_db(state.clone())
        .await?;
    let user_role = new_user
        .insert_user_role_in_db(
            state.clone(),
            consts::ROLE_ID_ORGANIZATION_ADMIN.to_string(),
            UserStatus::Active,
        )
        .await?;
    let jwt_token = user_from_db
        .get_jwt_auth_token(state.clone(), user_role.org_id)
        .await?;

    return Ok(ApplicationResponse::Json(api::SignUpResponse {
        token: Secret::new(jwt_token),
        merchant_id: user_role.merchant_id,
        name: user_from_db.get_name(),
        email: user_from_db.get_email(),
        verification_days_left: None,
        user_role: user_role.role_id,
        user_id: user_from_db.get_user_id().to_string(),
    }));
}

pub async fn signin(
    state: AppState,
    request: api::SignInRequest,
) -> UserResponse<api::SignInResponse> {
    let user_from_db: domain::UserFromStorage = state
        .store
        .find_user_by_email(request.email.clone().expose().expose().as_str())
        .await
        .map_err(|e| {
            if e.current_context().is_db_not_found() {
                e.change_context(UserErrors::InvalidCredentials)
            } else {
                e.change_context(UserErrors::InternalServerError)
            }
        })?
        .into();

    user_from_db.compare_password(request.password)?;

    let user_role = user_from_db.get_role_from_db(state.clone()).await?;
    let jwt_token = user_from_db
        .get_jwt_auth_token(state.clone(), user_role.org_id)
        .await?;

    return Ok(ApplicationResponse::Json(api::SignInResponse {
        token: Secret::new(jwt_token),
        merchant_id: user_role.merchant_id,
        name: user_from_db.get_name(),
        email: user_from_db.get_email(),
        verification_days_left: None,
        user_role: user_role.role_id,
        user_id: user_from_db.get_user_id().to_string(),
    }));
}
