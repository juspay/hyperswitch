use std::{collections::HashMap, sync::Arc};

use api_models::user as user_api;
use common_utils::errors::CustomResult;
use diesel_models::{enums::UserStatus, user_role::UserRole};
use error_stack::ResultExt;
use redis_interface::RedisConnectionPool;

use crate::{
    core::errors::{StorageError, UserErrors, UserResult},
    routes::SessionState,
    services::{
        authentication::{AuthToken, UserFromToken},
        authorization::roles::RoleInfo,
    },
    types::domain::{self, MerchantAccount, UserFromStorage},
};

pub mod dashboard_metadata;
pub mod password;
#[cfg(feature = "dummy_connector")]
pub mod sample_data;
pub mod two_factor_auth;

impl UserFromToken {
    pub async fn get_merchant_account_from_db(
        &self,
        state: SessionState,
    ) -> UserResult<MerchantAccount> {
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

    pub async fn get_user_from_db(&self, state: &SessionState) -> UserResult<UserFromStorage> {
        let user = state
            .global_store
            .find_user_by_id(&self.user_id)
            .await
            .change_context(UserErrors::InternalServerError)?;
        Ok(user.into())
    }

    pub async fn get_role_info_from_db(&self, state: &SessionState) -> UserResult<RoleInfo> {
        RoleInfo::from_role_id(state, &self.role_id, &self.merchant_id, &self.org_id)
            .await
            .change_context(UserErrors::InternalServerError)
    }
}

pub async fn generate_jwt_auth_token(
    state: &SessionState,
    user: &UserFromStorage,
    user_role: &UserRole,
) -> UserResult<masking::Secret<String>> {
    let token = AuthToken::new_token(
        user.get_user_id().to_string(),
        user_role.merchant_id.clone(),
        user_role.role_id.clone(),
        &state.conf,
        user_role.org_id.clone(),
    )
    .await?;
    Ok(masking::Secret::new(token))
}

pub async fn generate_jwt_auth_token_with_custom_role_attributes(
    state: &SessionState,
    user: &UserFromStorage,
    merchant_id: String,
    org_id: String,
    role_id: String,
) -> UserResult<masking::Secret<String>> {
    let token = AuthToken::new_token(
        user.get_user_id().to_string(),
        merchant_id,
        role_id,
        &state.conf,
        org_id,
    )
    .await?;
    Ok(masking::Secret::new(token))
}

pub fn get_dashboard_entry_response(
    state: &SessionState,
    user: UserFromStorage,
    user_role: UserRole,
    token: masking::Secret<String>,
) -> UserResult<user_api::DashboardEntryResponse> {
    let verification_days_left = get_verification_days_left(state, &user)?;

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

#[allow(unused_variables)]
pub fn get_verification_days_left(
    state: &SessionState,
    user: &UserFromStorage,
) -> UserResult<Option<i64>> {
    #[cfg(feature = "email")]
    return user.get_verification_days_left(state);
    #[cfg(not(feature = "email"))]
    return Ok(None);
}

pub fn get_multiple_merchant_details_with_status(
    user_roles: Vec<UserRole>,
    merchant_accounts: Vec<MerchantAccount>,
    roles: Vec<RoleInfo>,
) -> UserResult<Vec<user_api::UserMerchantAccount>> {
    let merchant_account_map = merchant_accounts
        .into_iter()
        .map(|merchant_account| (merchant_account.merchant_id.clone(), merchant_account))
        .collect::<HashMap<_, _>>();

    let role_map = roles
        .into_iter()
        .map(|role_info| (role_info.get_role_id().to_string(), role_info))
        .collect::<HashMap<_, _>>();

    user_roles
        .into_iter()
        .map(|user_role| {
            let merchant_account = merchant_account_map
                .get(&user_role.merchant_id)
                .ok_or(UserErrors::InternalServerError)
                .attach_printable("Merchant account for user role doesn't exist")?;

            let role_info = role_map
                .get(&user_role.role_id)
                .ok_or(UserErrors::InternalServerError)
                .attach_printable("Role info for user role doesn't exist")?;

            Ok(user_api::UserMerchantAccount {
                merchant_id: user_role.merchant_id,
                merchant_name: merchant_account.merchant_name.clone(),
                is_active: user_role.status == UserStatus::Active,
                role_id: user_role.role_id,
                role_name: role_info.get_role_name().to_string(),
                org_id: user_role.org_id,
            })
        })
        .collect()
}

pub async fn get_user_from_db_by_email(
    state: &SessionState,
    email: domain::UserEmail,
) -> CustomResult<UserFromStorage, StorageError> {
    state
        .global_store
        .find_user_by_email(&email.into_inner())
        .await
        .map(UserFromStorage::from)
}

pub fn get_token_from_signin_response(resp: &user_api::SignInResponse) -> masking::Secret<String> {
    match resp {
        user_api::SignInResponse::DashboardEntry(data) => data.token.clone(),
        user_api::SignInResponse::MerchantSelect(data) => data.token.clone(),
    }
}

pub fn get_redis_connection(state: &SessionState) -> UserResult<Arc<RedisConnectionPool>> {
    state
        .store
        .get_redis_conn()
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Failed to get redis connection")
}
