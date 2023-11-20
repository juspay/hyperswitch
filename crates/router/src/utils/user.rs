use crate::{
    core::errors::{UserResult, UserErrors}, routes::AppState, services::authentication::AuthToken,
    types::domain::UserFromStorage,
};
use diesel_models::user_role::UserRole;
use error_stack::ResultExt;
use masking::Secret;

pub mod password;

pub async fn generate_jwt_auth_token(
    state: AppState,
    user: &UserFromStorage,
    user_role: &UserRole,
) -> UserResult<Secret<String>> {
    let role_id = user.get_role_from_db(state.clone()).await?.role_id;
    let merchant_id = state
        .store
        .find_user_role_by_user_id(user.get_user_id())
        .await
        .change_context(UserErrors::InternalServerError)?
        .merchant_id;
    let token = AuthToken::new_token(
        user.get_user_id().to_string(),
        merchant_id,
        role_id,
        &state.conf,
        user_role.org_id.to_owned(),
    )
    .await?;
    Ok(Secret::new(token))
}
