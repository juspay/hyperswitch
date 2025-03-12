use std::sync::Arc;

use api_models::user as user_api;
use common_enums::UserAuthType;
use common_utils::{
    encryption::Encryption, errors::CustomResult, id_type, type_name, types::keymanager::Identifier,
};
use diesel_models::organization::{self, OrganizationBridge};
use error_stack::ResultExt;
use masking::{ExposeInterface, Secret};
use redis_interface::RedisConnectionPool;
use router_env::env;

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
pub mod theme;
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
        RoleInfo::from_role_id_org_id_tenant_id(
            state,
            &self.role_id,
            &self.org_id,
            self.tenant_id.as_ref().unwrap_or(&state.tenant.tenant_id),
        )
        .await
        .change_context(UserErrors::InternalServerError)
    }
}

pub async fn generate_jwt_auth_token_with_attributes(
    state: &SessionState,
    user_id: String,
    merchant_id: id_type::MerchantId,
    org_id: id_type::OrganizationId,
    role_id: String,
    profile_id: id_type::ProfileId,
    tenant_id: Option<id_type::TenantId>,
) -> UserResult<Secret<String>> {
    let token = AuthToken::new_token(
        user_id,
        merchant_id,
        role_id,
        &state.conf,
        org_id,
        profile_id,
        tenant_id,
    )
    .await?;
    Ok(Secret::new(token))
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

pub async fn get_user_from_db_by_email(
    state: &SessionState,
    email: domain::UserEmail,
) -> CustomResult<UserFromStorage, StorageError> {
    state
        .global_store
        .find_user_by_email(&email)
        .await
        .map(UserFromStorage::from)
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

            let encrypted_config =
                domain::types::crypto_operation::<serde_json::Value, masking::WithType>(
                    &state.into(),
                    type_name!(diesel_models::user::User),
                    domain::types::CryptoOperation::Encrypt(private_config_value.into()),
                    Identifier::UserAuth(id),
                    encryption_key,
                )
                .await
                .and_then(|val| val.try_into_operation())
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

    let private_config = domain::types::crypto_operation::<serde_json::Value, masking::WithType>(
        &state.into(),
        type_name!(diesel_models::user::User),
        domain::types::CryptoOperation::DecryptOptional(encrypted_config),
        Identifier::UserAuth(id),
        &user_auth_key,
    )
    .await
    .and_then(|val| val.try_into_optionaloperation())
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
        .set_key_with_expiry(&key.into(), sso_id, REDIS_SSO_TTL)
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
        .get_key::<Option<String>>(&key.into())
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

pub fn is_sso_auth_type(auth_type: UserAuthType) -> bool {
    match auth_type {
        UserAuthType::OpenIdConnect => true,
        UserAuthType::Password | UserAuthType::MagicLink => false,
    }
}

#[cfg(feature = "v1")]
pub fn create_merchant_account_request_for_org(
    req: user_api::UserOrgMerchantCreateRequest,
    org: organization::Organization,
    product_type: common_enums::MerchantProductType,
) -> UserResult<api_models::admin::MerchantAccountCreate> {
    let merchant_id = if matches!(env::which(), env::Env::Production) {
        id_type::MerchantId::try_from(domain::MerchantId::new(req.merchant_name.clone().expose())?)?
    } else {
        id_type::MerchantId::new_from_unix_timestamp()
    };

    let company_name = domain::UserCompanyName::new(req.merchant_name.expose())?;
    Ok(api_models::admin::MerchantAccountCreate {
        merchant_id,
        metadata: None,
        locker_id: None,
        return_url: None,
        merchant_name: Some(Secret::new(company_name.get_secret())),
        webhook_details: None,
        publishable_key: None,
        organization_id: Some(org.get_organization_id()),
        merchant_details: None,
        routing_algorithm: None,
        parent_merchant_id: None,
        sub_merchants_enabled: None,
        frm_routing_algorithm: None,
        #[cfg(feature = "payouts")]
        payout_routing_algorithm: None,
        primary_business_details: None,
        payment_response_hash_key: None,
        enable_payment_response_hash: None,
        redirect_to_merchant_with_http_post: None,
        pm_collect_link_config: None,
        product_type: Some(product_type),
    })
}

pub async fn validate_email_domain_auth_type_using_db(
    state: &SessionState,
    email: &domain::UserEmail,
    required_auth_type: UserAuthType,
) -> UserResult<()> {
    let domain = email.extract_domain()?;
    let user_auth_methods = state
        .store
        .list_user_authentication_methods_for_email_domain(domain)
        .await
        .change_context(UserErrors::InternalServerError)?;

    (user_auth_methods.is_empty()
        || user_auth_methods
            .iter()
            .any(|auth_method| auth_method.auth_type == required_auth_type))
    .then_some(())
    .ok_or(UserErrors::InvalidUserAuthMethodOperation.into())
}
