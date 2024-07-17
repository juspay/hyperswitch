use std::{collections::HashMap, sync::Arc};

use api_models::user as user_api;
use common_enums::UserAuthType;
use common_utils::{encryption::Encryption, errors::CustomResult, types::keymanager::Identifier};
use diesel_models::{enums::UserStatus, user_role::UserRole};
use error_stack::ResultExt;
use masking::{ExposeInterface, Secret};
use redis_interface::RedisConnectionPool;

use crate::{
    consts::user::{REDIS_SSO_PREFIX, REDIS_SSO_TTL},
    core::errors::{StorageError, UserErrors, UserResult},
    routes::SessionState,
    services::{
        authentication::{AuthToken, UserFromToken},
        authorization::roles::RoleInfo,
    },
    types::{
        domain::{self, MerchantAccount, UserFromStorage},
        transformers::ForeignFrom,
    },
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
        let key_manager_state = &(&state).into();
        let key_store = state
            .store
            .get_merchant_key_store_by_merchant_id(
                key_manager_state,
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
            .find_merchant_account_by_merchant_id(key_manager_state, &self.merchant_id, &key_store)
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

pub async fn generate_jwt_auth_token_with_custom_role_attributes(
    state: &SessionState,
    user: &UserFromStorage,
    merchant_id: String,
    org_id: String,
    role_id: String,
) -> UserResult<Secret<String>> {
    let token = AuthToken::new_token(
        user.get_user_id().to_string(),
        merchant_id,
        role_id,
        &state.conf,
        org_id,
    )
    .await?;
    Ok(Secret::new(token))
}

pub fn get_dashboard_entry_response(
    state: &SessionState,
    user: UserFromStorage,
    user_role: UserRole,
    token: Secret<String>,
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

pub fn get_token_from_signin_response(resp: &user_api::SignInResponse) -> Secret<String> {
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

impl ForeignFrom<&user_api::AuthConfig> for UserAuthType {
    fn foreign_from(from: &user_api::AuthConfig) -> Self {
        match *from {
            user_api::AuthConfig::OpenIdConnect { .. } => Self::OpenIdConnect,
            user_api::AuthConfig::Password => Self::Password,
            user_api::AuthConfig::MagicLink => Self::MagicLink,
        }
    }
}

pub async fn construct_public_and_private_db_configs(
    state: &SessionState,
    auth_config: &user_api::AuthConfig,
    encryption_key: &[u8],
    id: String,
) -> UserResult<(Option<Encryption>, Option<serde_json::Value>)> {
    match auth_config {
        user_api::AuthConfig::OpenIdConnect {
            private_config,
            public_config,
        } => {
            let private_config_value = serde_json::to_value(private_config.clone())
                .change_context(UserErrors::InternalServerError)
                .attach_printable("Failed to convert auth config to json")?;

            let encrypted_config = domain::types::encrypt::<serde_json::Value, masking::WithType>(
                &state.into(),
                private_config_value.into(),
                Identifier::UserAuth(id),
                encryption_key,
            )
            .await
            .change_context(UserErrors::InternalServerError)
            .attach_printable("Failed to encrypt auth config")?;

            Ok((
                Some(encrypted_config.into()),
                Some(
                    serde_json::to_value(public_config.clone())
                        .change_context(UserErrors::InternalServerError)
                        .attach_printable("Failed to convert auth config to json")?,
                ),
            ))
        }
        user_api::AuthConfig::Password | user_api::AuthConfig::MagicLink => Ok((None, None)),
    }
}

pub fn parse_value<T>(value: serde_json::Value, type_name: &str) -> UserResult<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value::<T>(value)
        .change_context(UserErrors::InternalServerError)
        .attach_printable(format!("Unable to parse {}", type_name))
}

pub async fn decrypt_oidc_private_config(
    state: &SessionState,
    encrypted_config: Option<Encryption>,
    id: String,
) -> UserResult<user_api::OpenIdConnectPrivateConfig> {
    let user_auth_key = hex::decode(
        state
            .conf
            .user_auth_methods
            .get_inner()
            .encryption_key
            .clone()
            .expose(),
    )
    .change_context(UserErrors::InternalServerError)
    .attach_printable("Failed to decode DEK")?;

    let private_config = domain::types::decrypt::<serde_json::Value, masking::WithType>(
        &state.into(),
        encrypted_config,
        Identifier::UserAuth(id),
        &user_auth_key,
    )
    .await
    .change_context(UserErrors::InternalServerError)
    .attach_printable("Failed to decrypt private config")?
    .ok_or(UserErrors::InternalServerError)
    .attach_printable("Private config not found")?
    .into_inner()
    .expose();

    serde_json::from_value::<user_api::OpenIdConnectPrivateConfig>(private_config)
        .change_context(UserErrors::InternalServerError)
        .attach_printable("unable to parse OpenIdConnectPrivateConfig")
}

pub async fn set_sso_id_in_redis(
    state: &SessionState,
    oidc_state: Secret<String>,
    sso_id: String,
) -> UserResult<()> {
    let connection = get_redis_connection(state)?;
    let key = get_oidc_key(&oidc_state.expose());
    connection
        .set_key_with_expiry(&key, sso_id, REDIS_SSO_TTL)
        .await
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Failed to set sso id in redis")
}

pub async fn get_sso_id_from_redis(
    state: &SessionState,
    oidc_state: Secret<String>,
) -> UserResult<String> {
    let connection = get_redis_connection(state)?;
    let key = get_oidc_key(&oidc_state.expose());
    connection
        .get_key::<Option<String>>(&key)
        .await
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Failed to get sso id from redis")?
        .ok_or(UserErrors::SSOFailed)
        .attach_printable("Cannot find oidc state in redis. Oidc state invalid or expired")
}

fn get_oidc_key(oidc_state: &str) -> String {
    format!("{}{oidc_state}", REDIS_SSO_PREFIX)
}

pub fn get_oidc_sso_redirect_url(state: &SessionState, provider: &str) -> String {
    format!("{}/redirect/oidc/{}", state.conf.user.base_url, provider)
}

pub fn is_sso_auth_type(auth_type: &UserAuthType) -> bool {
    match auth_type {
        UserAuthType::OpenIdConnect => true,
        UserAuthType::Password | UserAuthType::MagicLink => false,
    }
}
