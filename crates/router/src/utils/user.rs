use api_models::user as user_api;
use diesel_models::{enums::UserStatus, user_role::UserRole};
use error_stack::ResultExt;
use masking::Secret;

use crate::{
    core::errors::{UserErrors, UserResult},
    routes::AppState,
    services::authentication::{AuthToken, UserFromToken},
    types::domain::{MerchantAccount, UserFromStorage},
};

pub mod dashboard_metadata;
pub mod password;
#[cfg(feature = "dummy_connector")]
pub mod sample_data;

impl UserFromToken {
    pub async fn get_merchant_account(&self, state: AppState) -> UserResult<MerchantAccount> {
        let key_store = state
            .store
            .get_merchant_key_store_by_merchant_id(
                &self.merchant_id,
                &state.store.get_master_key().to_vec().into(),
            )
            .await
            .map_err(|e| {
                if e.current_context().is_db_not_found() {
                    e.change_context(UserErrors::MerchantIdNotFound)
                } else {
                    e.change_context(UserErrors::InternalServerError)
                }
            })?;
        let merchant_account = state
            .store
            .find_merchant_account_by_merchant_id(&self.merchant_id, &key_store)
            .await
            .map_err(|e| {
                if e.current_context().is_db_not_found() {
                    e.change_context(UserErrors::MerchantIdNotFound)
                } else {
                    e.change_context(UserErrors::InternalServerError)
                }
            })?;
        Ok(merchant_account)
    }

    pub async fn get_user(&self, state: AppState) -> UserResult<diesel_models::user::User> {
        let user = state
            .store
            .find_user_by_id(&self.user_id)
            .await
            .change_context(UserErrors::InternalServerError)?;
        Ok(user)
    }
}

pub async fn get_merchant_ids_for_user(state: AppState, user_id: &str) -> UserResult<Vec<String>> {
    Ok(state
        .store
        .list_user_roles_by_user_id(user_id)
        .await
        .change_context(UserErrors::InternalServerError)?
        .into_iter()
        .filter_map(|ele| {
            if ele.status == UserStatus::Active {
                return Some(ele.merchant_id);
            }
            None
        })
        .collect())
}

pub async fn generate_jwt_auth_token(
    state: AppState,
    user: &UserFromStorage,
    user_role: &UserRole,
) -> UserResult<Secret<String>> {
    let token = AuthToken::new_token(
        user.get_user_id().to_string(),
        user_role.merchant_id.clone(),
        user_role.role_id.clone(),
        &state.conf,
        user_role.org_id.clone(),
    )
    .await?;
    Ok(Secret::new(token))
}

pub async fn generate_jwt_auth_token_with_custom_merchant_id(
    state: AppState,
    user: &UserFromStorage,
    user_role: &UserRole,
    merchant_id: String,
) -> UserResult<Secret<String>> {
    let token = AuthToken::new_token(
        user.get_user_id().to_string(),
        merchant_id,
        user_role.role_id.clone(),
        &state.conf,
        user_role.org_id.to_owned(),
    )
    .await?;
    Ok(Secret::new(token))
}

#[allow(unused_variables)]
pub fn get_dashboard_entry_response(
    state: AppState,
    user: UserFromStorage,
    user_role: UserRole,
    token: Secret<String>,
) -> UserResult<user_api::DashboardEntryResponse> {
    #[cfg(feature = "email")]
    let verification_days_left = user.get_verification_days_left(state)?;
    #[cfg(not(feature = "email"))]
    let verification_days_left = None;

    Ok(user_api::DashboardEntryResponse {
        merchant_id: user_role.merchant_id,
        token,
        name: user.get_name(),
        email: user.get_email(),
        user_id: user.get_user_id().to_string(),
        verification_days_left,
        user_role: user_role.role_id,
    })
}
