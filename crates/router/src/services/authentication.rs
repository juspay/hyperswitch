use std::{marker::PhantomData, str::FromStr};

use actix_web::http::header::HeaderMap;
#[cfg(feature = "v2")]
use api_models::payment_methods::PaymentMethodIntentConfirm;
#[cfg(feature = "v1")]
use api_models::payment_methods::{PaymentMethodCreate, PaymentMethodListRequest};
use api_models::payments;
#[cfg(feature = "payouts")]
use api_models::payouts;
use async_trait::async_trait;
use base64::Engine;
use common_enums::{MerchantAccountType, TokenPurpose};
use common_utils::{date_time, fp_utils, id_type};
#[cfg(feature = "v2")]
use diesel_models::ephemeral_key;
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::sdk_auth::SdkAuthorization;
use jsonwebtoken::{
    decode, errors::ErrorKind::ExpiredSignature, Algorithm, DecodingKey, Validation,
};
#[cfg(feature = "v2")]
use masking::ExposeInterface;
use masking::PeekInterface;
use router_env::logger;
use serde::Serialize;

use self::blacklist::BlackList;
#[cfg(all(feature = "partial-auth", feature = "v1"))]
use self::detached::ExtractedPayload;
#[cfg(feature = "partial-auth")]
use self::detached::GetAuthType;
use super::authorization::{self, permissions::Permission};
#[cfg(feature = "olap")]
use super::jwt;
#[cfg(feature = "olap")]
use crate::configs::Settings;
#[cfg(feature = "olap")]
use crate::consts;
#[cfg(feature = "olap")]
use crate::core::errors::UserResult;
#[cfg(all(feature = "partial-auth", feature = "v1"))]
use crate::core::metrics;
use crate::{
    configs::settings,
    consts::BASE64_ENGINE,
    core::{
        api_keys,
        errors::{self, utils::StorageErrorExt, RouterResult},
    },
    headers,
    routes::app::SessionStateInfo,
    services::api,
    types::{domain, storage},
    utils::OptionExt,
};

pub mod blacklist;
pub mod cookies;
pub mod decision;
pub mod embedded;

#[cfg(feature = "partial-auth")]
mod detached;

#[cfg(feature = "v1")]
#[derive(Clone, Debug)]
pub struct AuthenticationData {
    pub platform: domain::Platform,
    pub profile: Option<domain::Profile>,
    pub client_secret: Option<String>,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug)]
pub struct AuthenticationData {
    pub platform: domain::Platform,
    pub profile: domain::Profile,
    pub client_secret: Option<String>,
}

#[cfg(feature = "v1")]
impl AuthenticationData {
    pub fn construct_authentication_data_for_internal_merchant_id_profile_id_auth(
        platform: domain::Platform,
        profile: domain::Profile,
    ) -> Self {
        Self {
            platform,
            profile: Some(profile),
            client_secret: None,
        }
    }
}

#[cfg(feature = "v2")]
impl AuthenticationData {
    pub fn construct_authentication_data_for_internal_merchant_id_profile_id_auth(
        platform: domain::Platform,
        profile: domain::Profile,
    ) -> Self {
        Self {
            platform,
            profile,
            client_secret: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PlatformAccountWithKeyStore {
    account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
}

#[derive(Clone, Debug)]
pub struct AuthenticationDataWithoutProfile {
    pub merchant_account: domain::MerchantAccount,
    pub key_store: domain::MerchantKeyStore,
}

#[derive(Clone, Debug)]
pub struct AuthenticationDataWithMultipleProfiles {
    pub merchant_account: domain::MerchantAccount,
    pub key_store: domain::MerchantKeyStore,
    pub profile_id_list: Option<Vec<id_type::ProfileId>>,
}

#[derive(Clone, Debug)]
pub struct AuthenticationDataWithUser {
    pub merchant_account: domain::MerchantAccount,
    pub key_store: domain::MerchantKeyStore,
    pub user: storage::User,
    pub profile_id: id_type::ProfileId,
}

#[derive(Clone, Debug)]
pub struct AuthenticationDataWithOrg {
    pub organization_id: id_type::OrganizationId,
}

#[derive(Clone)]
pub struct UserFromTokenWithRoleInfo {
    pub user: UserFromToken,
    pub role_info: authorization::roles::RoleInfo,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(
    tag = "api_auth_type",
    content = "authentication_data",
    rename_all = "snake_case"
)]
pub enum AuthenticationType {
    ApiKey {
        merchant_id: id_type::MerchantId,
        key_id: id_type::ApiKeyId,
    },
    AdminApiKey,
    AdminApiAuthWithMerchantId {
        merchant_id: id_type::MerchantId,
    },
    OrganizationJwt {
        org_id: id_type::OrganizationId,
        user_id: String,
    },
    BasicAuth {
        username: String,
    },
    MerchantJwt {
        merchant_id: id_type::MerchantId,
        user_id: Option<String>,
    },
    MerchantJwtWithProfileId {
        merchant_id: id_type::MerchantId,
        profile_id: Option<id_type::ProfileId>,
        user_id: String,
    },
    UserJwt {
        user_id: String,
    },
    SinglePurposeJwt {
        user_id: String,
        purpose: TokenPurpose,
    },
    SinglePurposeOrLoginJwt {
        user_id: String,
        purpose: Option<TokenPurpose>,
        role_id: Option<String>,
    },
    MerchantId {
        merchant_id: id_type::MerchantId,
    },
    PublishableKey {
        merchant_id: id_type::MerchantId,
    },
    SdkAuthorization {
        merchant_id: id_type::MerchantId,
    },
    WebhookAuth {
        merchant_id: id_type::MerchantId,
    },
    InternalMerchantIdProfileId {
        merchant_id: id_type::MerchantId,
        profile_id: Option<id_type::ProfileId>,
    },
    EmbeddedJwt {
        merchant_id: id_type::MerchantId,
        profile_id: id_type::ProfileId,
    },
    NoAuth,
}

impl events::EventInfo for AuthenticationType {
    type Data = Self;
    fn data(&self) -> error_stack::Result<Self::Data, events::EventsError> {
        Ok(self.clone())
    }

    fn key(&self) -> String {
        "auth_info".to_string()
    }
}

impl AuthenticationType {
    pub fn get_merchant_id(&self) -> Option<&id_type::MerchantId> {
        match self {
            Self::ApiKey {
                merchant_id,
                key_id: _,
            }
            | Self::AdminApiAuthWithMerchantId { merchant_id }
            | Self::MerchantId { merchant_id }
            | Self::PublishableKey { merchant_id }
            | Self::MerchantJwt {
                merchant_id,
                user_id: _,
            }
            | Self::MerchantJwtWithProfileId { merchant_id, .. }
            | Self::WebhookAuth { merchant_id }
            | Self::InternalMerchantIdProfileId { merchant_id, .. }
            | Self::EmbeddedJwt { merchant_id, .. }
            | Self::SdkAuthorization { merchant_id, .. } => Some(merchant_id),
            Self::AdminApiKey
            | Self::OrganizationJwt { .. }
            | Self::BasicAuth { .. }
            | Self::UserJwt { .. }
            | Self::SinglePurposeJwt { .. }
            | Self::SinglePurposeOrLoginJwt { .. }
            | Self::NoAuth => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, serde::Deserialize, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ExternalServiceType {
    Hypersense,
}

#[cfg(feature = "olap")]
#[derive(Clone, Debug)]
pub struct UserFromSinglePurposeToken {
    pub user_id: String,
    pub origin: domain::Origin,
    pub path: Vec<TokenPurpose>,
    pub tenant_id: Option<id_type::TenantId>,
}

#[cfg(feature = "olap")]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SinglePurposeToken {
    pub user_id: String,
    pub purpose: TokenPurpose,
    pub origin: domain::Origin,
    pub path: Vec<TokenPurpose>,
    pub exp: u64,
    pub tenant_id: Option<id_type::TenantId>,
}

#[cfg(feature = "olap")]
impl SinglePurposeToken {
    pub async fn new_token(
        user_id: String,
        purpose: TokenPurpose,
        origin: domain::Origin,
        settings: &Settings,
        path: Vec<TokenPurpose>,
        tenant_id: Option<id_type::TenantId>,
    ) -> UserResult<String> {
        let exp_duration =
            std::time::Duration::from_secs(consts::SINGLE_PURPOSE_TOKEN_TIME_IN_SECS);
        let exp = jwt::generate_exp(exp_duration)?.as_secs();
        let token_payload = Self {
            user_id,
            purpose,
            origin,
            exp,
            path,
            tenant_id,
        };
        jwt::generate_jwt(&token_payload, settings).await
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct AuthToken {
    pub user_id: String,
    pub merchant_id: id_type::MerchantId,
    pub role_id: String,
    pub exp: u64,
    pub org_id: id_type::OrganizationId,
    pub profile_id: id_type::ProfileId,
    pub tenant_id: Option<id_type::TenantId>,
}

#[cfg(feature = "olap")]
impl AuthToken {
    pub async fn new_token(
        user_id: String,
        merchant_id: id_type::MerchantId,
        role_id: String,
        settings: &Settings,
        org_id: id_type::OrganizationId,
        profile_id: id_type::ProfileId,
        tenant_id: Option<id_type::TenantId>,
    ) -> UserResult<String> {
        let exp_duration = std::time::Duration::from_secs(consts::JWT_TOKEN_TIME_IN_SECS);
        let exp = jwt::generate_exp(exp_duration)?.as_secs();
        let token_payload = Self {
            user_id,
            merchant_id,
            role_id,
            exp,
            org_id,
            profile_id,
            tenant_id,
        };
        jwt::generate_jwt(&token_payload, settings).await
    }
}

#[derive(Clone)]
pub struct UserFromToken {
    pub user_id: String,
    pub merchant_id: id_type::MerchantId,
    pub role_id: String,
    pub org_id: id_type::OrganizationId,
    pub profile_id: id_type::ProfileId,
    pub tenant_id: Option<id_type::TenantId>,
}

pub struct UserIdFromAuth {
    pub user_id: String,
    pub tenant_id: Option<id_type::TenantId>,
}

#[cfg(feature = "olap")]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SinglePurposeOrLoginToken {
    pub user_id: String,
    pub role_id: Option<String>,
    pub purpose: Option<TokenPurpose>,
    pub exp: u64,
    pub tenant_id: Option<id_type::TenantId>,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
pub enum AuthOrEmbeddedClaims {
    AuthToken(AuthToken),
    EmbeddedToken(embedded::EmbeddedToken),
}

impl AuthOrEmbeddedClaims {
    fn get_tenant_id(&self) -> Option<&id_type::TenantId> {
        match self {
            Self::AuthToken(payload) => payload.tenant_id.as_ref(),
            Self::EmbeddedToken(payload) => Some(&payload.tenant_id),
        }
    }

    fn get_merchant_id(&self) -> &id_type::MerchantId {
        match self {
            Self::AuthToken(payload) => &payload.merchant_id,
            Self::EmbeddedToken(payload) => &payload.merchant_id,
        }
    }

    fn get_profile_id(&self) -> &id_type::ProfileId {
        match self {
            Self::AuthToken(payload) => &payload.profile_id,
            Self::EmbeddedToken(payload) => &payload.profile_id,
        }
    }
}

#[async_trait]
pub trait AuthenticateAndFetch<T, A>
where
    A: SessionStateInfo,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(T, AuthenticationType)>;
}

#[derive(Debug, Default)]
pub struct ApiKeyAuth {
    pub allow_connected_scope_operation: bool,
    pub allow_platform_self_operation: bool,
}

pub struct NoAuth;

pub trait BasicAuthProvider {
    type Identity;

    fn get_credentials<A>(
        state: &A,
        identifier: &str,
    ) -> RouterResult<(Self::Identity, masking::Secret<String>)>
    where
        A: SessionStateInfo;
}

#[derive(Debug, Default)]
pub struct BasicAuth<P> {
    _marker: PhantomData<P>,
}

impl<P> BasicAuth<P> {
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

pub struct OidcAuthProvider;

impl BasicAuthProvider for OidcAuthProvider {
    type Identity = String;

    fn get_credentials<A>(
        state: &A,
        identifier: &str,
    ) -> RouterResult<(Self::Identity, masking::Secret<String>)>
    where
        A: SessionStateInfo,
    {
        let session = state.session_state();
        let client = session
            .conf
            .oidc
            .get_inner()
            .get_client(identifier)
            .ok_or(errors::ApiErrorResponse::InvalidBasicAuth)?;

        Ok((client.client_id.clone(), client.client_secret.clone()))
    }
}

pub const OIDC_CLIENT_AUTH: BasicAuth<OidcAuthProvider> = BasicAuth::<OidcAuthProvider>::new();

#[cfg(feature = "partial-auth")]
impl GetAuthType for ApiKeyAuth {
    fn get_auth_type(&self) -> detached::PayloadType {
        detached::PayloadType::ApiKey
    }
}

#[cfg(all(feature = "partial-auth", feature = "v2"))]
impl GetAuthType for V2ApiKeyAuth {
    fn get_auth_type(&self) -> detached::PayloadType {
        detached::PayloadType::ApiKey
    }
}

#[cfg(feature = "partial-auth")]
pub trait GetMerchantAccessFlags {
    fn is_connected_scope_operation_allowed(&self) -> bool;
    fn is_platform_self_operation_allowed(&self) -> bool;
}

#[cfg(feature = "partial-auth")]
impl GetMerchantAccessFlags for ApiKeyAuth {
    fn is_connected_scope_operation_allowed(&self) -> bool {
        self.allow_connected_scope_operation
    }
    fn is_platform_self_operation_allowed(&self) -> bool {
        self.allow_platform_self_operation
    }
}

#[cfg(all(feature = "partial-auth", feature = "v2"))]
impl GetMerchantAccessFlags for V2ApiKeyAuth {
    fn is_connected_scope_operation_allowed(&self) -> bool {
        self.allow_connected_scope_operation
    }
    fn is_platform_self_operation_allowed(&self) -> bool {
        self.allow_platform_self_operation
    }
}

//
// # Header Auth
//
// Header Auth is a feature that allows you to authenticate requests using custom headers. This is
// done by checking whether the request contains the specified headers.
// - `x-merchant-id` header is used to authenticate the merchant.
//
// ## Checksum
// - `x-auth-checksum` header is used to authenticate the request. The checksum is calculated using the
// above mentioned headers is generated by hashing the headers mentioned above concatenated with `:` and then hashed with the detached authentication key.
//
// When the [`partial-auth`] feature is disabled the implementation for [`AuthenticateAndFetch`]
// changes where the authentication is done by the [`I`] implementation.
//
pub struct HeaderAuth<I>(pub I);

#[async_trait]
impl<A> AuthenticateAndFetch<(), A> for NoAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        _request_headers: &HeaderMap,
        _state: &A,
    ) -> RouterResult<((), AuthenticationType)> {
        Ok(((), AuthenticationType::NoAuth))
    }
}

#[async_trait]
impl<A, T> AuthenticateAndFetch<Option<T>, A> for NoAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        _request_headers: &HeaderMap,
        _state: &A,
    ) -> RouterResult<(Option<T>, AuthenticationType)> {
        Ok((None, AuthenticationType::NoAuth))
    }
}

#[async_trait]
impl<A, P> AuthenticateAndFetch<P::Identity, A> for BasicAuth<P>
where
    A: SessionStateInfo + Sync,
    P: BasicAuthProvider + Send + Sync,
    P::Identity: Clone,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(P::Identity, AuthenticationType)> {
        let (provided_identifier, provided_secret) = parse_basic_auth_credentials(request_headers)?;

        let (authenticated_entity, expected_secret) =
            P::get_credentials(state, &provided_identifier)?;

        if provided_secret.peek() != expected_secret.peek() {
            return Err(errors::ApiErrorResponse::InvalidBasicAuth.into());
        }

        Ok((
            authenticated_entity.clone(),
            AuthenticationType::BasicAuth {
                username: provided_identifier,
            },
        ))
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for ApiKeyAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let api_key = get_api_key(request_headers)
            .change_context(errors::ApiErrorResponse::Unauthorized)?
            .trim();
        if api_key.is_empty() {
            return Err(errors::ApiErrorResponse::Unauthorized)
                .attach_printable("API key is empty");
        }

        let profile_id = HeaderMapStruct::new(request_headers)
            .get_id_type_from_header::<id_type::ProfileId>(headers::X_PROFILE_ID)?;

        let api_key = api_keys::PlaintextApiKey::from(api_key);
        let hash_key = {
            let config = state.conf();
            config.api_keys.get_inner().get_hash_key()?
        };
        let hashed_api_key = api_key.keyed_hash(hash_key.peek());

        let stored_api_key = state
            .store()
            .find_api_key_by_hash_optional(hashed_api_key.into())
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError) // If retrieve failed
            .attach_printable("Failed to retrieve API key")?
            .ok_or(report!(errors::ApiErrorResponse::Unauthorized)) // If retrieve returned `None`
            .attach_printable("Merchant not authenticated")?;

        if stored_api_key
            .expires_at
            .map(|expires_at| expires_at < date_time::now())
            .unwrap_or(false)
        {
            return Err(report!(errors::ApiErrorResponse::Unauthorized))
                .attach_printable("API key has expired");
        }

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &stored_api_key.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .change_context(errors::ApiErrorResponse::Unauthorized)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let initiator_merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&stored_api_key.merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let profile = state
            .store()
            .find_business_profile_by_profile_id(&key_store, &profile_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        // Validate access based on merchant type and header presence
        check_merchant_access(
            state,
            request_headers,
            initiator_merchant.merchant_account_type,
            self.allow_connected_scope_operation,
            self.allow_platform_self_operation,
        )?;

        let initiator = Some(domain::Initiator::Api {
            merchant_id: initiator_merchant.get_id().clone(),
            merchant_account_type: initiator_merchant.merchant_account_type,
            publishable_key: initiator_merchant.publishable_key.clone(),
        });

        let platform = resolve_platform(
            state,
            request_headers,
            initiator_merchant.clone(),
            key_store,
            initiator,
        )
        .await?;

        let auth = AuthenticationData {
            platform,
            profile,
            client_secret: None,
        };
        Ok((
            auth.clone(),
            AuthenticationType::ApiKey {
                merchant_id: initiator_merchant.get_id().clone(),
                key_id: stored_api_key.key_id,
            },
        ))
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for ApiKeyAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let api_key = get_api_key(request_headers)
            .change_context(errors::ApiErrorResponse::Unauthorized)?
            .trim();
        if api_key.is_empty() {
            return Err(errors::ApiErrorResponse::Unauthorized)
                .attach_printable("API key is empty");
        }

        let api_key = api_keys::PlaintextApiKey::from(api_key);
        let hash_key = {
            let config = state.conf();
            config.api_keys.get_inner().get_hash_key()?
        };
        let hashed_api_key = api_key.keyed_hash(hash_key.peek());

        let stored_api_key = state
            .store()
            .find_api_key_by_hash_optional(hashed_api_key.into())
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError) // If retrieve failed
            .attach_printable("Failed to retrieve API key")?
            .ok_or(report!(errors::ApiErrorResponse::Unauthorized)) // If retrieve returned `None`
            .attach_printable("Merchant not authenticated")?;

        if stored_api_key
            .expires_at
            .map(|expires_at| expires_at < date_time::now())
            .unwrap_or(false)
        {
            return Err(report!(errors::ApiErrorResponse::Unauthorized))
                .attach_printable("API key has expired");
        }

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &stored_api_key.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .change_context(errors::ApiErrorResponse::Unauthorized)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let profile_id =
            get_header_value_by_key(headers::X_PROFILE_ID.to_string(), request_headers)?
                .map(id_type::ProfileId::from_str)
                .transpose()
                .change_context(errors::ValidationError::IncorrectValueProvided {
                    field_name: "X-Profile-Id",
                })
                .change_context(errors::ApiErrorResponse::Unauthorized)?;

        let initiator_merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&stored_api_key.merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        // Validate access based on merchant type and header presence
        check_merchant_access(
            state,
            request_headers,
            initiator_merchant.merchant_account_type,
            self.allow_connected_scope_operation,
            self.allow_platform_self_operation,
        )?;

        let initiator = Some(domain::Initiator::Api {
            merchant_id: initiator_merchant.get_id().clone(),
            merchant_account_type: initiator_merchant.merchant_account_type,
            publishable_key: initiator_merchant.publishable_key.clone(),
        });

        let platform = resolve_platform(
            state,
            request_headers,
            initiator_merchant.clone(),
            key_store.clone(),
            initiator,
        )
        .await?;

        let profile = match profile_id {
            Some(profile_id) => {
                let profile = state
                    .store()
                    .find_business_profile_by_profile_id(
                        platform.get_processor().get_key_store(),
                        &profile_id,
                    )
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;
                Some(profile)
            }
            None => None,
        };

        let auth = AuthenticationData {
            platform,
            profile,
            client_secret: None,
        };
        Ok((
            auth.clone(),
            AuthenticationType::ApiKey {
                merchant_id: initiator_merchant.get_id().clone(),
                key_id: stored_api_key.key_id,
            },
        ))
    }
}

#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationDataWithUserId, A> for ApiKeyAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationDataWithUserId, AuthenticationType)> {
        let (auth_data, auth_type): (AuthenticationData, AuthenticationType) =
            self.authenticate_and_fetch(request_headers, state).await?;

        Ok(((auth_data, None), auth_type))
    }
}

#[derive(Debug)]
pub struct ApiKeyAuthWithMerchantIdFromRoute(pub id_type::MerchantId);

#[cfg(feature = "partial-auth")]
impl GetAuthType for ApiKeyAuthWithMerchantIdFromRoute {
    fn get_auth_type(&self) -> detached::PayloadType {
        detached::PayloadType::ApiKey
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for ApiKeyAuthWithMerchantIdFromRoute
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        // This is currently used for profile and connector CRUD operations
        let api_auth = ApiKeyAuth {
            allow_connected_scope_operation: true,
            allow_platform_self_operation: false,
        };
        let (auth_data, auth_type): (AuthenticationData, AuthenticationType) = api_auth
            .authenticate_and_fetch(request_headers, state)
            .await?;

        let merchant_id_from_route = self.0.clone();
        let processor_merchant_id = auth_data.platform.get_processor().get_account().get_id();

        if merchant_id_from_route != *processor_merchant_id {
            return Err(report!(errors::ApiErrorResponse::Unauthorized))
                .attach_printable("Merchant ID from route and Processor Merchant Id do not match");
        }

        Ok((auth_data, auth_type))
    }
}

#[derive(Debug, Default)]
pub struct PlatformOrgAdminAuth {
    pub is_admin_auth_allowed: bool,
    pub organization_id: Option<id_type::OrganizationId>,
}

#[cfg(feature = "v1")]
#[async_trait]
impl<A> AuthenticateAndFetch<Option<AuthenticationDataWithOrg>, A> for PlatformOrgAdminAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(Option<AuthenticationDataWithOrg>, AuthenticationType)> {
        // Step 1: Admin API Key and API Key Fallback (if allowed)
        if self.is_admin_auth_allowed {
            let admin_auth = AdminApiAuthWithApiKeyFallback {
                organization_id: self.organization_id.clone(),
            };
            match admin_auth
                .authenticate_and_fetch(request_headers, state)
                .await
            {
                Ok((auth, auth_type)) => {
                    return Ok((auth, auth_type));
                }
                Err(e) => {
                    logger::warn!("Admin API Auth failed: {:?}", e);
                }
            }
        }

        // Step 2: Try Platform Auth
        let api_key = get_api_key(request_headers)
            .change_context(errors::ApiErrorResponse::Unauthorized)?
            .trim();
        if api_key.is_empty() {
            return Err(errors::ApiErrorResponse::Unauthorized)
                .attach_printable("API key is empty");
        }

        let api_key_plaintext = api_keys::PlaintextApiKey::from(api_key);
        let hash_key = state.conf().api_keys.get_inner().get_hash_key()?;
        let hashed_api_key = api_key_plaintext.keyed_hash(hash_key.peek());

        let stored_api_key = state
            .store()
            .find_api_key_by_hash_optional(hashed_api_key.into())
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to retrieve API key")?
            .ok_or_else(|| report!(errors::ApiErrorResponse::Unauthorized))
            .attach_printable("Merchant not authenticated via API key")?;

        if stored_api_key
            .expires_at
            .map(|expires_at| expires_at < date_time::now())
            .unwrap_or(false)
        {
            return Err(report!(errors::ApiErrorResponse::Unauthorized))
                .attach_printable("API key has expired");
        }

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &stored_api_key.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .change_context(errors::ApiErrorResponse::Unauthorized)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let merchant_account = state
            .store()
            .find_merchant_account_by_merchant_id(&stored_api_key.merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)
            .attach_printable("Merchant account not found")?;

        if !(state.conf().platform.enabled && merchant_account.is_platform_account()) {
            return Err(report!(errors::ApiErrorResponse::Unauthorized)
                .attach_printable("Platform authentication check failed"));
        }

        fp_utils::when(
            self.organization_id
                .as_ref()
                .is_some_and(|org_id| org_id != merchant_account.get_org_id()),
            || {
                Err(report!(errors::ApiErrorResponse::Unauthorized))
                    .attach_printable("Organization ID does not match")
            },
        )?;

        Ok((
            Some(AuthenticationDataWithOrg {
                organization_id: merchant_account.get_org_id().clone(),
            }),
            AuthenticationType::ApiKey {
                merchant_id: merchant_account.get_id().clone(),
                key_id: stored_api_key.key_id,
            },
        ))
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for PlatformOrgAdminAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let api_key = get_api_key(request_headers)
            .change_context(errors::ApiErrorResponse::Unauthorized)?
            .trim();
        if api_key.is_empty() {
            return Err(errors::ApiErrorResponse::Unauthorized)
                .attach_printable("API key is empty");
        }

        let api_key_plaintext = api_keys::PlaintextApiKey::from(api_key);
        let hash_key = state.conf().api_keys.get_inner().get_hash_key()?;
        let hashed_api_key = api_key_plaintext.keyed_hash(hash_key.peek());

        let stored_api_key = state
            .store()
            .find_api_key_by_hash_optional(hashed_api_key.into())
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to retrieve API key")?
            .ok_or_else(|| report!(errors::ApiErrorResponse::Unauthorized))
            .attach_printable("Merchant not authenticated via API key")?;

        if stored_api_key
            .expires_at
            .map(|expires_at| expires_at < date_time::now())
            .unwrap_or(false)
        {
            return Err(report!(errors::ApiErrorResponse::Unauthorized))
                .attach_printable("API key has expired");
        }

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &stored_api_key.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .change_context(errors::ApiErrorResponse::Unauthorized)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let initiator_merchant_account = state
            .store()
            .find_merchant_account_by_merchant_id(&stored_api_key.merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)
            .attach_printable("Merchant account not found")?;

        if !(state.conf().platform.enabled && initiator_merchant_account.is_platform_account()) {
            return Err(report!(errors::ApiErrorResponse::Unauthorized)
                .attach_printable("Platform authentication check failed"));
        }

        fp_utils::when(
            self.organization_id
                .as_ref()
                .is_some_and(|org_id| org_id != initiator_merchant_account.get_org_id()),
            || {
                Err(report!(errors::ApiErrorResponse::Unauthorized))
                    .attach_printable("Organization ID does not match")
            },
        )?;

        let initiator = Some(domain::Initiator::Api {
            merchant_id: initiator_merchant_account.get_id().clone(),
            merchant_account_type: initiator_merchant_account.merchant_account_type,
            publishable_key: initiator_merchant_account.publishable_key.clone(),
        });

        let platform = domain::Platform::new(
            initiator_merchant_account.clone(),
            key_store.clone(),
            initiator_merchant_account.clone(),
            key_store,
            initiator,
        );

        let auth = AuthenticationData {
            platform,
            profile: None,
            client_secret: None,
        };

        Ok((
            auth.clone(),
            AuthenticationType::ApiKey {
                merchant_id: initiator_merchant_account.get_id().clone(),
                key_id: stored_api_key.key_id,
            },
        ))
    }
}

#[derive(Debug)]
pub struct PlatformOrgAdminAuthWithMerchantIdFromRoute {
    pub merchant_id_from_route: id_type::MerchantId,
    pub is_admin_auth_allowed: bool,
}

#[cfg(feature = "v1")]
impl PlatformOrgAdminAuthWithMerchantIdFromRoute {
    async fn fetch_key_store_and_account<A: SessionStateInfo + Sync>(
        merchant_id: &id_type::MerchantId,
        state: &A,
    ) -> RouterResult<(domain::MerchantKeyStore, domain::MerchantAccount)> {
        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        Ok((key_store, merchant))
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for PlatformOrgAdminAuthWithMerchantIdFromRoute
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let route_merchant_id = self.merchant_id_from_route.clone();

        // Step 1: Admin API Key and API Key Fallback (if allowed)
        if self.is_admin_auth_allowed {
            let admin_auth =
                AdminApiAuthWithApiKeyFallbackAndMerchantIdFromRoute(route_merchant_id.clone());

            match admin_auth
                .authenticate_and_fetch(request_headers, state)
                .await
            {
                Ok((auth_data, auth_type)) => return Ok((auth_data, auth_type)),
                Err(e) => {
                    logger::warn!("Admin API Auth failed: {:?}", e);
                }
            }
        }

        // Step 2: Platform authentication
        let api_key = get_api_key(request_headers)
            .change_context(errors::ApiErrorResponse::Unauthorized)?
            .trim();
        if api_key.is_empty() {
            return Err(errors::ApiErrorResponse::Unauthorized)
                .attach_printable("API key is empty");
        }

        let api_key_plaintext = api_keys::PlaintextApiKey::from(api_key);
        let hash_key = {
            let config = state.conf();
            config.api_keys.get_inner().get_hash_key()?
        };
        let hashed_api_key = api_key_plaintext.keyed_hash(hash_key.peek());

        let stored_api_key = state
            .store()
            .find_api_key_by_hash_optional(hashed_api_key.into())
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to retrieve API key")?
            .ok_or_else(|| report!(errors::ApiErrorResponse::Unauthorized))
            .attach_printable("Merchant not authenticated via API key")?;

        if stored_api_key
            .expires_at
            .map(|expires_at| expires_at < date_time::now())
            .unwrap_or(false)
        {
            return Err(report!(errors::ApiErrorResponse::Unauthorized))
                .attach_printable("API key has expired");
        }

        let (platform_key_store, platform_merchant) =
            Self::fetch_key_store_and_account(&stored_api_key.merchant_id, state).await?;

        if !(state.conf().platform.enabled && platform_merchant.is_platform_account()) {
            return Err(report!(errors::ApiErrorResponse::Unauthorized))
                .attach_printable("Platform authentication check failed");
        }

        let (route_key_store, route_merchant) =
            Self::fetch_key_store_and_account(&route_merchant_id, state).await?;

        if platform_merchant.get_org_id() != route_merchant.get_org_id() {
            return Err(report!(errors::ApiErrorResponse::Unauthorized))
                .attach_printable("Route merchant not under same org as platform merchant");
        }

        let initiator = Some(domain::Initiator::Api {
            merchant_id: platform_merchant.get_id().clone(),
            merchant_account_type: platform_merchant.merchant_account_type,
            publishable_key: platform_merchant.publishable_key.clone(),
        });

        let platform = domain::Platform::new(
            platform_merchant.clone(),
            platform_key_store.clone(),
            route_merchant,
            route_key_store,
            initiator,
        );

        let auth = AuthenticationData {
            platform,
            profile: None,
            client_secret: None,
        };

        Ok((
            auth.clone(),
            AuthenticationType::ApiKey {
                merchant_id: platform_merchant.get_id().clone(),
                key_id: stored_api_key.key_id,
            },
        ))
    }
}

#[cfg(not(feature = "partial-auth"))]
#[async_trait]
impl<A, I> AuthenticateAndFetch<AuthenticationData, A> for HeaderAuth<I>
where
    A: SessionStateInfo + Send + Sync,
    I: AuthenticateAndFetch<AuthenticationData, A> + Sync + Send,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        self.0.authenticate_and_fetch(request_headers, state).await
    }
}

#[cfg(all(feature = "partial-auth", feature = "v1"))]
#[async_trait]
impl<A, I> AuthenticateAndFetch<AuthenticationData, A> for HeaderAuth<I>
where
    A: SessionStateInfo + Sync,
    I: AuthenticateAndFetch<AuthenticationData, A>
        + GetAuthType
        + GetMerchantAccessFlags
        + Sync
        + Send,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let enable_partial_auth = state.conf().api_keys.get_inner().enable_partial_auth;

        // This is a early return if partial auth is disabled
        // Preventing the need to go through the header extraction process
        if !enable_partial_auth {
            return self.0.authenticate_and_fetch(request_headers, state).await;
        }

        let report_failure = || {
            metrics::PARTIAL_AUTH_FAILURE.add(1, &[]);
        };

        let profile_id = HeaderMapStruct::new(request_headers)
            .get_id_type_from_header_if_present::<id_type::ProfileId>(headers::X_PROFILE_ID)
            .change_context(errors::ValidationError::IncorrectValueProvided {
                field_name: "X-Profile-Id",
            })
            .change_context(errors::ApiErrorResponse::Unauthorized)?;

        let payload = ExtractedPayload::from_headers(request_headers)
            .and_then(|value| {
                let (algo, secret) = state.get_detached_auth()?;

                Ok(value
                    .verify_checksum(request_headers, algo, secret)
                    .then_some(value))
            })
            .map(|inner_payload| {
                inner_payload.and_then(|inner| {
                    (inner.payload_type == self.0.get_auth_type()).then_some(inner)
                })
            });

        match payload {
            Ok(Some(data)) => match data {
                ExtractedPayload {
                    payload_type: detached::PayloadType::ApiKey,
                    merchant_id: Some(merchant_id),
                    key_id: Some(key_id),
                } => {
                    let auth = construct_authentication_data(
                        state,
                        &merchant_id,
                        request_headers,
                        profile_id,
                        self.0.is_connected_scope_operation_allowed(),
                        self.0.is_platform_self_operation_allowed(),
                    )
                    .await?;
                    Ok((
                        auth,
                        AuthenticationType::ApiKey {
                            merchant_id: merchant_id.clone(),
                            key_id,
                        },
                    ))
                }
                ExtractedPayload {
                    payload_type: detached::PayloadType::PublishableKey,
                    merchant_id: Some(merchant_id),
                    key_id: None,
                } => {
                    let auth = construct_authentication_data(
                        state,
                        &merchant_id,
                        request_headers,
                        profile_id,
                        self.0.is_connected_scope_operation_allowed(),
                        self.0.is_platform_self_operation_allowed(),
                    )
                    .await?;
                    Ok((
                        auth,
                        AuthenticationType::PublishableKey {
                            merchant_id: merchant_id.clone(),
                        },
                    ))
                }
                _ => {
                    report_failure();
                    self.0.authenticate_and_fetch(request_headers, state).await
                }
            },
            Ok(None) => {
                report_failure();
                self.0.authenticate_and_fetch(request_headers, state).await
            }
            Err(error) => {
                logger::error!(%error, "Failed to extract payload from headers");
                report_failure();
                self.0.authenticate_and_fetch(request_headers, state).await
            }
        }
    }
}

#[cfg(all(feature = "partial-auth", feature = "v2"))]
#[async_trait]
impl<A, I> AuthenticateAndFetch<AuthenticationData, A> for HeaderAuth<I>
where
    A: SessionStateInfo + Sync,
    I: AuthenticateAndFetch<AuthenticationData, A>
        + AuthenticateAndFetch<AuthenticationData, A>
        + GetAuthType
        + GetMerchantAccessFlags
        + Sync
        + Send,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let (auth_data, auth_type): (AuthenticationData, AuthenticationType) = self
            .0
            .authenticate_and_fetch(request_headers, state)
            .await?;

        let profile_id = HeaderMapStruct::new(request_headers)
            .get_id_type_from_header::<id_type::ProfileId>(headers::X_PROFILE_ID)?;

        let profile = state
            .store()
            .find_business_profile_by_profile_id(
                auth_data.platform.get_processor().get_key_store(),
                &profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let auth = AuthenticationData {
            platform: auth_data.platform,
            profile,
            client_secret: None,
        };
        Ok((auth, auth_type))
    }
}

#[cfg(all(feature = "partial-auth", feature = "v1"))]
async fn construct_authentication_data<A>(
    state: &A,
    merchant_id: &id_type::MerchantId,
    request_headers: &HeaderMap,
    profile_id: Option<id_type::ProfileId>,
    allow_connected_scope_operation: bool,
    allow_platform_self_operation: bool,
) -> RouterResult<AuthenticationData>
where
    A: SessionStateInfo + Sync,
{
    let key_store = state
        .store()
        .get_merchant_key_store_by_merchant_id(
            merchant_id,
            &state.store().get_master_key().to_vec().into(),
        )
        .await
        .change_context(errors::ApiErrorResponse::Unauthorized)
        .attach_printable("Failed to fetch merchant key store for the merchant id")?;

    let initiator_merchant = state
        .store()
        .find_merchant_account_by_merchant_id(merchant_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

    // Validate access based on merchant type and header presence
    check_merchant_access(
        state,
        request_headers,
        initiator_merchant.merchant_account_type,
        allow_connected_scope_operation,
        allow_platform_self_operation,
    )?;

    let initiator = Some(domain::Initiator::Api {
        merchant_id: initiator_merchant.get_id().clone(),
        merchant_account_type: initiator_merchant.merchant_account_type,
        publishable_key: initiator_merchant.publishable_key.clone(),
    });

    let platform = resolve_platform(
        state,
        request_headers,
        initiator_merchant,
        key_store,
        initiator,
    )
    .await?;

    let profile = match profile_id {
        Some(profile_id) => {
            let profile = state
                .store()
                .find_business_profile_by_profile_id(
                    platform.get_processor().get_key_store(),
                    &profile_id,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;
            Some(profile)
        }
        None => None,
    };

    let auth = AuthenticationData {
        platform,
        profile,
        client_secret: None,
    };

    Ok(auth)
}

#[cfg(feature = "olap")]
#[derive(Debug)]
pub(crate) struct SinglePurposeJWTAuth(pub TokenPurpose);

#[cfg(feature = "olap")]
#[async_trait]
impl<A> AuthenticateAndFetch<UserFromSinglePurposeToken, A> for SinglePurposeJWTAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(UserFromSinglePurposeToken, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, SinglePurposeToken>(request_headers, state).await?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }
        authorization::check_tenant(
            payload.tenant_id.clone(),
            &state.session_state().tenant.tenant_id,
        )?;

        if self.0 != payload.purpose {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }

        Ok((
            UserFromSinglePurposeToken {
                user_id: payload.user_id.clone(),
                origin: payload.origin.clone(),
                path: payload.path,
                tenant_id: payload.tenant_id,
            },
            AuthenticationType::SinglePurposeJwt {
                user_id: payload.user_id,
                purpose: payload.purpose,
            },
        ))
    }
}

#[cfg(feature = "olap")]
#[async_trait]
impl<A> AuthenticateAndFetch<Option<UserFromSinglePurposeToken>, A> for SinglePurposeJWTAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(Option<UserFromSinglePurposeToken>, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, SinglePurposeToken>(request_headers, state).await?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }
        authorization::check_tenant(
            payload.tenant_id.clone(),
            &state.session_state().tenant.tenant_id,
        )?;

        if self.0 != payload.purpose {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }

        Ok((
            Some(UserFromSinglePurposeToken {
                user_id: payload.user_id.clone(),
                origin: payload.origin.clone(),
                path: payload.path,
                tenant_id: payload.tenant_id,
            }),
            AuthenticationType::SinglePurposeJwt {
                user_id: payload.user_id,
                purpose: payload.purpose,
            },
        ))
    }
}

#[cfg(feature = "olap")]
#[derive(Debug)]
pub struct SinglePurposeOrLoginTokenAuth(pub TokenPurpose);

#[cfg(feature = "olap")]
#[async_trait]
impl<A> AuthenticateAndFetch<UserIdFromAuth, A> for SinglePurposeOrLoginTokenAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(UserIdFromAuth, AuthenticationType)> {
        let payload =
            parse_jwt_payload::<A, SinglePurposeOrLoginToken>(request_headers, state).await?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }
        authorization::check_tenant(
            payload.tenant_id.clone(),
            &state.session_state().tenant.tenant_id,
        )?;

        let is_purpose_equal = payload
            .purpose
            .as_ref()
            .is_some_and(|purpose| purpose == &self.0);

        let purpose_exists = payload.purpose.is_some();
        let role_id_exists = payload.role_id.is_some();

        if is_purpose_equal && !role_id_exists || role_id_exists && !purpose_exists {
            Ok((
                UserIdFromAuth {
                    user_id: payload.user_id.clone(),
                    tenant_id: payload.tenant_id,
                },
                AuthenticationType::SinglePurposeOrLoginJwt {
                    user_id: payload.user_id,
                    purpose: payload.purpose,
                    role_id: payload.role_id,
                },
            ))
        } else {
            Err(errors::ApiErrorResponse::InvalidJwtToken.into())
        }
    }
}

#[cfg(feature = "olap")]
#[derive(Debug)]
pub struct AnyPurposeOrLoginTokenAuth;

#[cfg(feature = "olap")]
#[async_trait]
impl<A> AuthenticateAndFetch<UserIdFromAuth, A> for AnyPurposeOrLoginTokenAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(UserIdFromAuth, AuthenticationType)> {
        let payload =
            parse_jwt_payload::<A, SinglePurposeOrLoginToken>(request_headers, state).await?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }

        let purpose_exists = payload.purpose.is_some();
        let role_id_exists = payload.role_id.is_some();

        if purpose_exists ^ role_id_exists {
            Ok((
                UserIdFromAuth {
                    user_id: payload.user_id.clone(),
                    tenant_id: payload.tenant_id,
                },
                AuthenticationType::SinglePurposeOrLoginJwt {
                    user_id: payload.user_id,
                    purpose: payload.purpose,
                    role_id: payload.role_id,
                },
            ))
        } else {
            Err(errors::ApiErrorResponse::InvalidJwtToken.into())
        }
    }
}

#[derive(Debug, Default)]
pub struct AdminApiAuth;

#[async_trait]
impl<A> AuthenticateAndFetch<(), A> for AdminApiAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<((), AuthenticationType)> {
        let request_admin_api_key =
            get_api_key(request_headers).change_context(errors::ApiErrorResponse::Unauthorized)?;
        let conf = state.conf();

        let admin_api_key = &conf.secrets.get_inner().admin_api_key;

        if request_admin_api_key != admin_api_key.peek() {
            Err(report!(errors::ApiErrorResponse::Unauthorized)
                .attach_printable("Admin Authentication Failure"))?;
        }

        Ok(((), AuthenticationType::AdminApiKey))
    }
}

#[derive(Debug, Default)]
pub struct V2AdminApiAuth;

#[async_trait]
impl<A> AuthenticateAndFetch<(), A> for V2AdminApiAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<((), AuthenticationType)> {
        let header_map_struct = HeaderMapStruct::new(request_headers);
        let auth_string = header_map_struct.get_auth_string_from_header()?;
        let request_admin_api_key = auth_string
            .split(',')
            .find_map(|part| part.trim().strip_prefix("admin-api-key="))
            .ok_or_else(|| {
                report!(errors::ApiErrorResponse::Unauthorized)
                    .attach_printable("Unable to parse admin_api_key")
            })?;
        if request_admin_api_key.is_empty() {
            return Err(errors::ApiErrorResponse::Unauthorized)
                .attach_printable("Admin Api key is empty");
        }
        let conf = state.conf();

        let admin_api_key = &conf.secrets.get_inner().admin_api_key;

        if request_admin_api_key != admin_api_key.peek() {
            Err(report!(errors::ApiErrorResponse::Unauthorized)
                .attach_printable("Admin Authentication Failure"))?;
        }

        Ok(((), AuthenticationType::AdminApiKey))
    }
}
#[derive(Debug)]
pub struct AdminApiAuthWithMerchantIdFromRoute(pub id_type::MerchantId);

#[cfg(feature = "v1")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for AdminApiAuthWithMerchantIdFromRoute
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        AdminApiAuth
            .authenticate_and_fetch(request_headers, state)
            .await?;

        let merchant_id = self.0.clone();

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let initiator = Some(domain::Initiator::Admin);

        let platform = domain::Platform::new(
            merchant.clone(),
            key_store.clone(),
            merchant,
            key_store,
            initiator,
        );

        let auth = AuthenticationData {
            platform,
            profile: None,
            client_secret: None,
        };

        Ok((
            auth,
            AuthenticationType::AdminApiAuthWithMerchantId { merchant_id },
        ))
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for AdminApiAuthWithMerchantIdFromRoute
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        if state.conf().platform.enabled {
            throw_error_if_platform_merchant_authentication_required(request_headers)?;
        }

        V2AdminApiAuth
            .authenticate_and_fetch(request_headers, state)
            .await?;

        let merchant_id = self.0.clone();
        let profile_id =
            get_id_type_by_key_from_headers(headers::X_PROFILE_ID.to_string(), request_headers)?
                .get_required_value(headers::X_PROFILE_ID)?;
        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;
        let profile = state
            .store()
            .find_business_profile_by_merchant_id_profile_id(&key_store, &merchant_id, &profile_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;
        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let initiator = Some(domain::Initiator::Admin);

        let platform = domain::Platform::new(
            merchant.clone(),
            key_store.clone(),
            merchant,
            key_store,
            initiator,
        );

        let auth = AuthenticationData {
            platform,
            profile,
            client_secret: None,
        };
        Ok((
            auth,
            AuthenticationType::AdminApiAuthWithMerchantId { merchant_id },
        ))
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationDataWithoutProfile, A>
    for AdminApiAuthWithMerchantIdFromRoute
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationDataWithoutProfile, AuthenticationType)> {
        if state.conf().platform.enabled {
            throw_error_if_platform_merchant_authentication_required(request_headers)?;
        }

        V2AdminApiAuth
            .authenticate_and_fetch(request_headers, state)
            .await?;

        let merchant_id = self.0.clone();

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let auth = AuthenticationDataWithoutProfile {
            merchant_account: merchant,
            key_store,
        };

        Ok((
            auth,
            AuthenticationType::AdminApiAuthWithMerchantId { merchant_id },
        ))
    }
}

#[derive(Debug, Default)]
pub struct AdminApiAuthWithApiKeyFallback {
    pub organization_id: Option<id_type::OrganizationId>,
}

#[cfg(feature = "v1")]
#[async_trait]
impl<A> AuthenticateAndFetch<Option<AuthenticationDataWithOrg>, A>
    for AdminApiAuthWithApiKeyFallback
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(Option<AuthenticationDataWithOrg>, AuthenticationType)> {
        let request_api_key =
            get_api_key(request_headers).change_context(errors::ApiErrorResponse::Unauthorized)?;

        let conf = state.conf();

        let admin_api_key = &conf.secrets.get_inner().admin_api_key;

        if request_api_key == admin_api_key.peek() {
            return Ok((None, AuthenticationType::AdminApiKey));
        }
        let Some(fallback_merchant_ids) = conf.fallback_merchant_ids_api_key_auth.as_ref() else {
            return Err(report!(errors::ApiErrorResponse::Unauthorized)).attach_printable(
                "Api Key Authentication Failure: fallback merchant set not configured",
            );
        };

        let api_key = api_keys::PlaintextApiKey::from(request_api_key);
        let hash_key = conf.api_keys.get_inner().get_hash_key()?;
        let hashed_api_key = api_key.keyed_hash(hash_key.peek());

        let stored_api_key = state
            .store()
            .find_api_key_by_hash_optional(hashed_api_key.into())
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to retrieve API key")?
            .ok_or(report!(errors::ApiErrorResponse::Unauthorized))
            .attach_printable("Merchant not authenticated")?;

        if stored_api_key
            .expires_at
            .map(|expires_at| expires_at < date_time::now())
            .unwrap_or(false)
        {
            return Err(report!(errors::ApiErrorResponse::Unauthorized))
                .attach_printable("API key has expired");
        }

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &stored_api_key.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .change_context(errors::ApiErrorResponse::Unauthorized)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&stored_api_key.merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        fp_utils::when(
            self.organization_id
                .as_ref()
                .is_some_and(|org_id| org_id != merchant.get_org_id()),
            || {
                Err(
                    report!(errors::ApiErrorResponse::Unauthorized).attach_printable(
                        "Organization ID from request and merchant account does not match",
                    ),
                )
            },
        )?;

        if fallback_merchant_ids
            .merchant_ids
            .contains(&stored_api_key.merchant_id)
        {
            return Ok((
                Some(AuthenticationDataWithOrg {
                    organization_id: merchant.organization_id,
                }),
                AuthenticationType::ApiKey {
                    merchant_id: stored_api_key.merchant_id,
                    key_id: stored_api_key.key_id,
                },
            ));
        }
        Err(report!(errors::ApiErrorResponse::Unauthorized)
            .attach_printable("Admin Authentication Failure"))
    }
}

#[derive(Debug, Default)]
pub struct AdminApiAuthWithApiKeyFallbackAndMerchantIdFromRoute(pub id_type::MerchantId);

#[cfg(feature = "v1")]
impl AdminApiAuthWithApiKeyFallbackAndMerchantIdFromRoute {
    async fn fetch_merchant_key_store_and_account<A: SessionStateInfo + Sync>(
        merchant_id: &id_type::MerchantId,
        state: &A,
    ) -> RouterResult<(domain::MerchantKeyStore, domain::MerchantAccount)> {
        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        Ok((key_store, merchant))
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A>
    for AdminApiAuthWithApiKeyFallbackAndMerchantIdFromRoute
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let merchant_id_from_route: id_type::MerchantId = self.0.clone();
        let request_api_key =
            get_api_key(request_headers).change_context(errors::ApiErrorResponse::Unauthorized)?;
        let conf = state.conf();

        let admin_api_key: &masking::Secret<String> = &conf.secrets.get_inner().admin_api_key;

        if request_api_key == admin_api_key.peek() {
            let (key_store, merchant) =
                Self::fetch_merchant_key_store_and_account(&merchant_id_from_route, state).await?;

            let initiator = Some(domain::Initiator::Admin);

            let platform = domain::Platform::new(
                merchant.clone(),
                key_store.clone(),
                merchant,
                key_store,
                initiator,
            );

            let auth = AuthenticationData {
                platform,
                profile: None,
                client_secret: None,
            };
            return Ok((
                auth,
                AuthenticationType::AdminApiAuthWithMerchantId {
                    merchant_id: merchant_id_from_route.clone(),
                },
            ));
        }

        let Some(fallback_merchant_ids) = conf.fallback_merchant_ids_api_key_auth.as_ref() else {
            return Err(report!(errors::ApiErrorResponse::Unauthorized)).attach_printable(
                "Api Key Authentication Failure: fallback merchant set not configured",
            );
        };

        let api_key = api_keys::PlaintextApiKey::from(request_api_key);
        let hash_key = conf.api_keys.get_inner().get_hash_key()?;
        let hashed_api_key = api_key.keyed_hash(hash_key.peek());

        let stored_api_key = state
            .store()
            .find_api_key_by_hash_optional(hashed_api_key.into())
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to retrieve API key")?
            .ok_or(report!(errors::ApiErrorResponse::Unauthorized))
            .attach_printable("Merchant not authenticated")?;

        if stored_api_key
            .expires_at
            .map(|expires_at| expires_at < date_time::now())
            .unwrap_or(false)
        {
            return Err(report!(errors::ApiErrorResponse::Unauthorized))
                .attach_printable("API key has expired");
        }

        if fallback_merchant_ids
            .merchant_ids
            .contains(&stored_api_key.merchant_id)
        {
            let (api_key_store, api_key_merchant) =
                Self::fetch_merchant_key_store_and_account(&stored_api_key.merchant_id, state)
                    .await?;
            let (route_key_store, route_merchant) =
                Self::fetch_merchant_key_store_and_account(&merchant_id_from_route, state).await?;
            if api_key_merchant.get_org_id() == route_merchant.get_org_id() {
                let initiator = Some(domain::Initiator::Api {
                    merchant_id: api_key_merchant.get_id().clone(),
                    merchant_account_type: api_key_merchant.merchant_account_type,
                    publishable_key: api_key_merchant.publishable_key.clone(),
                });

                let platform = domain::Platform::new(
                    api_key_merchant.clone(),
                    api_key_store.clone(),
                    route_merchant,
                    route_key_store,
                    initiator,
                );

                let auth = AuthenticationData {
                    platform,
                    profile: None,
                    client_secret: None,
                };
                return Ok((
                    auth,
                    AuthenticationType::MerchantId {
                        merchant_id: api_key_merchant.get_id().clone(),
                    },
                ));
            }
        }

        Err(report!(errors::ApiErrorResponse::Unauthorized)
            .attach_printable("Admin Authentication Failure"))
    }
}

/// A helper struct to extract headers from the request
pub(crate) struct HeaderMapStruct<'a> {
    headers: &'a HeaderMap,
}

impl<'a> HeaderMapStruct<'a> {
    pub fn new(headers: &'a HeaderMap) -> Self {
        HeaderMapStruct { headers }
    }

    fn get_mandatory_header_value_by_key(
        &self,
        key: &str,
    ) -> Result<&str, error_stack::Report<errors::ApiErrorResponse>> {
        self.headers
            .get(key)
            .ok_or(errors::ApiErrorResponse::InvalidRequestData {
                message: format!("Missing header key: `{key}`"),
            })
            .attach_printable(format!("Failed to find header key: {key}"))?
            .to_str()
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "`{key}` in headers",
            })
            .attach_printable(format!(
                "Failed to convert header value to string for header key: {key}",
            ))
    }

    /// Get the id type from the header
    /// This can be used to extract lineage ids from the headers
    pub fn get_id_type_from_header<
        T: TryFrom<
            std::borrow::Cow<'static, str>,
            Error = error_stack::Report<errors::ValidationError>,
        >,
    >(
        &self,
        key: &str,
    ) -> RouterResult<T> {
        self.get_mandatory_header_value_by_key(key)
            .map(|val| val.to_owned())
            .and_then(|header_value| {
                T::try_from(std::borrow::Cow::Owned(header_value)).change_context(
                    errors::ApiErrorResponse::InvalidRequestData {
                        message: format!("`{key}` header is invalid"),
                    },
                )
            })
    }
    #[cfg(feature = "v2")]
    pub fn get_organization_id_from_header(&self) -> RouterResult<id_type::OrganizationId> {
        self.get_mandatory_header_value_by_key(headers::X_ORGANIZATION_ID)
            .map(|val| val.to_owned())
            .and_then(|organization_id| {
                id_type::OrganizationId::try_from_string(organization_id).change_context(
                    errors::ApiErrorResponse::InvalidRequestData {
                        message: format!("`{}` header is invalid", headers::X_ORGANIZATION_ID),
                    },
                )
            })
    }

    pub fn get_header_value_by_key(&self, key: &str) -> Option<&str> {
        self.headers.get(key).and_then(|value| value.to_str().ok())
    }

    pub fn get_auth_string_from_header(&self) -> RouterResult<&str> {
        self.headers
            .get(headers::AUTHORIZATION)
            .get_required_value(headers::AUTHORIZATION)?
            .to_str()
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: headers::AUTHORIZATION,
            })
            .attach_printable("Failed to convert authorization header to string")
    }

    pub fn get_id_type_from_header_if_present<T>(&self, key: &str) -> RouterResult<Option<T>>
    where
        T: TryFrom<
            std::borrow::Cow<'static, str>,
            Error = error_stack::Report<errors::ValidationError>,
        >,
    {
        self.headers
            .get(key)
            .map(|value| value.to_str())
            .transpose()
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "`{key}` in headers",
            })
            .attach_printable(format!(
                "Failed to convert header value to string for header key: {key}",
            ))?
            .map(|value| {
                T::try_from(std::borrow::Cow::Owned(value.to_owned())).change_context(
                    errors::ApiErrorResponse::InvalidRequestData {
                        message: format!("`{key}` header is invalid"),
                    },
                )
            })
            .transpose()
    }
}

/// Get the merchant-id from `x-merchant-id` header
#[derive(Debug)]
pub struct AdminApiAuthWithMerchantIdFromHeader;

#[cfg(feature = "v1")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for AdminApiAuthWithMerchantIdFromHeader
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        AdminApiAuth
            .authenticate_and_fetch(request_headers, state)
            .await?;

        let merchant_id = HeaderMapStruct::new(request_headers)
            .get_id_type_from_header::<id_type::MerchantId>(headers::X_MERCHANT_ID)?;

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let initiator = Some(domain::Initiator::Admin);

        let platform = domain::Platform::new(
            merchant.clone(),
            key_store.clone(),
            merchant,
            key_store,
            initiator,
        );

        let auth = AuthenticationData {
            platform,
            profile: None,
            client_secret: None,
        };
        Ok((
            auth,
            AuthenticationType::AdminApiAuthWithMerchantId { merchant_id },
        ))
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for AdminApiAuthWithMerchantIdFromHeader
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        if state.conf().platform.enabled {
            throw_error_if_platform_merchant_authentication_required(request_headers)?;
        }

        V2AdminApiAuth
            .authenticate_and_fetch(request_headers, state)
            .await?;

        let merchant_id = HeaderMapStruct::new(request_headers)
            .get_id_type_from_header::<id_type::MerchantId>(headers::X_MERCHANT_ID)?;
        let profile_id =
            get_id_type_by_key_from_headers(headers::X_PROFILE_ID.to_string(), request_headers)?
                .get_required_value(headers::X_PROFILE_ID)?;

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;
        let profile = state
            .store()
            .find_business_profile_by_merchant_id_profile_id(&key_store, &merchant_id, &profile_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;
        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let initiator = Some(domain::Initiator::Admin);

        let platform = domain::Platform::new(
            merchant.clone(),
            key_store.clone(),
            merchant,
            key_store,
            initiator,
        );

        let auth = AuthenticationData {
            platform,
            profile,
            client_secret: None,
        };
        Ok((
            auth,
            AuthenticationType::AdminApiAuthWithMerchantId { merchant_id },
        ))
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationDataWithoutProfile, A>
    for AdminApiAuthWithMerchantIdFromHeader
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationDataWithoutProfile, AuthenticationType)> {
        if state.conf().platform.enabled {
            throw_error_if_platform_merchant_authentication_required(request_headers)?;
        }

        V2AdminApiAuth
            .authenticate_and_fetch(request_headers, state)
            .await?;

        let merchant_id = HeaderMapStruct::new(request_headers)
            .get_id_type_from_header::<id_type::MerchantId>(headers::X_MERCHANT_ID)?;

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let auth = AuthenticationDataWithoutProfile {
            merchant_account: merchant,
            key_store,
        };
        Ok((
            auth,
            AuthenticationType::AdminApiAuthWithMerchantId { merchant_id },
        ))
    }
}

#[derive(Debug)]
pub struct EphemeralKeyAuth;

#[cfg(feature = "v1")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for EphemeralKeyAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let api_key =
            get_api_key(request_headers).change_context(errors::ApiErrorResponse::Unauthorized)?;
        let ephemeral_key = state
            .store()
            .get_ephemeral_key(api_key)
            .await
            .change_context(errors::ApiErrorResponse::Unauthorized)?;

        MerchantIdAuth(ephemeral_key.merchant_id)
            .authenticate_and_fetch(request_headers, state)
            .await
    }
}

#[derive(Debug)]
#[cfg(feature = "v1")]
pub struct MerchantIdAuth(pub id_type::MerchantId);

#[cfg(feature = "v1")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for MerchantIdAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        if state.conf().platform.enabled {
            throw_error_if_platform_merchant_authentication_required(request_headers)?;
        }

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &self.0,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&self.0, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let initiator = Some(domain::Initiator::Api {
            merchant_id: merchant.get_id().clone(),
            merchant_account_type: merchant.merchant_account_type,
            publishable_key: merchant.publishable_key.clone(),
        });

        let platform =
            resolve_platform(state, request_headers, merchant, key_store, initiator).await?;

        let auth = AuthenticationData {
            platform,
            profile: None,
            client_secret: None,
        };
        Ok((
            auth.clone(),
            AuthenticationType::MerchantId {
                merchant_id: self.0.clone(),
            },
        ))
    }
}

#[derive(Debug)]
#[cfg(feature = "v2")]
pub struct MerchantIdAuth;

#[cfg(feature = "v2")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for MerchantIdAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        if state.conf().platform.enabled {
            throw_error_if_platform_merchant_authentication_required(request_headers)?;
        }

        let merchant_id = HeaderMapStruct::new(request_headers)
            .get_id_type_from_header::<id_type::MerchantId>(headers::X_MERCHANT_ID)?;
        let profile_id =
            get_id_type_by_key_from_headers(headers::X_PROFILE_ID.to_string(), request_headers)?
                .get_required_value(headers::X_PROFILE_ID)?;
        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let profile = state
            .store()
            .find_business_profile_by_merchant_id_profile_id(&key_store, &merchant_id, &profile_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;
        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let initiator = Some(domain::Initiator::Api {
            merchant_id: merchant.get_id().clone(),
            merchant_account_type: merchant.merchant_account_type,
            publishable_key: merchant.publishable_key.clone(),
        });

        let platform = domain::Platform::new(
            merchant.clone(),
            key_store.clone(),
            merchant.clone(),
            key_store,
            initiator,
        );

        let auth = AuthenticationData {
            platform,
            profile,
            client_secret: None,
        };
        Ok((
            auth.clone(),
            AuthenticationType::MerchantId {
                merchant_id: merchant.get_id().clone(),
            },
        ))
    }
}

/// InternalMerchantIdProfileIdAuth authentication which first tries to authenticate using `X-Internal-API-Key`,
/// `X-Merchant-Id` and `X-Profile-Id` headers. If any of these headers are missing,
/// it falls back to the provided authentication mechanism.
pub struct InternalMerchantIdProfileIdAuth<F>(pub F);

pub fn is_internal_merchant_id_profile_id_auth(
    request_headers: &HeaderMap,
) -> common_enums::ApiKeyType {
    let merchant_id = HeaderMapStruct::new(request_headers)
        .get_id_type_from_header::<id_type::MerchantId>(headers::X_MERCHANT_ID)
        .ok();
    let internal_api_key = HeaderMapStruct::new(request_headers)
        .get_header_value_by_key(headers::X_INTERNAL_API_KEY)
        .map(|internal_api_key| internal_api_key.to_string());
    let profile_id = HeaderMapStruct::new(request_headers)
        .get_id_type_from_header::<id_type::ProfileId>(headers::X_PROFILE_ID)
        .ok();

    if merchant_id.is_some() && profile_id.is_some() && internal_api_key.is_some() {
        common_enums::ApiKeyType::Internal
    } else {
        common_enums::ApiKeyType::External
    }
}

#[async_trait]
impl<A, F> AuthenticateAndFetch<AuthenticationData, A> for InternalMerchantIdProfileIdAuth<F>
where
    A: SessionStateInfo + Sync + Send,
    F: AuthenticateAndFetch<AuthenticationData, A> + Sync + Send,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        if !state.conf().internal_merchant_id_profile_id_auth.enabled {
            return self.0.authenticate_and_fetch(request_headers, state).await;
        }
        let merchant_id = HeaderMapStruct::new(request_headers)
            .get_id_type_from_header::<id_type::MerchantId>(headers::X_MERCHANT_ID)
            .ok();
        let internal_api_key = HeaderMapStruct::new(request_headers)
            .get_header_value_by_key(headers::X_INTERNAL_API_KEY)
            .map(|s| s.to_string());
        let profile_id = HeaderMapStruct::new(request_headers)
            .get_id_type_from_header::<id_type::ProfileId>(headers::X_PROFILE_ID)
            .ok();
        if let (Some(internal_api_key), Some(merchant_id), Some(profile_id)) =
            (internal_api_key, merchant_id, profile_id)
        {
            let config = state.conf();
            if internal_api_key
                != *config
                    .internal_merchant_id_profile_id_auth
                    .internal_api_key
                    .peek()
            {
                return Err(errors::ApiErrorResponse::Unauthorized)
                    .attach_printable("Internal API key authentication failed");
            }
            let key_store = state
                .store()
                .get_merchant_key_store_by_merchant_id(
                    &merchant_id,
                    &state.store().get_master_key().to_vec().into(),
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

            let profile = state
                .store()
                .find_business_profile_by_merchant_id_profile_id(
                    &key_store,
                    &merchant_id,
                    &profile_id,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

            let initiator_merchant = state
                .store()
                .find_merchant_account_by_merchant_id(&merchant_id, &key_store)
                .await
                .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

            let initiator = Some(domain::Initiator::Api {
                merchant_id: initiator_merchant.get_id().clone(),
                merchant_account_type: initiator_merchant.merchant_account_type,
                publishable_key: initiator_merchant.publishable_key.clone(),
            });

            let platform = resolve_platform(
                state,
                request_headers,
                initiator_merchant.clone(),
                key_store,
                initiator,
            )
            .await?;

            let auth = AuthenticationData::construct_authentication_data_for_internal_merchant_id_profile_id_auth(platform, profile);

            Ok((
                auth.clone(),
                AuthenticationType::InternalMerchantIdProfileId {
                    merchant_id: merchant_id.clone(),
                    profile_id: Some(profile_id),
                },
            ))
        } else {
            Ok(self
                .0
                .authenticate_and_fetch(request_headers, state)
                .await?)
        }
    }
}

#[derive(Debug)]
#[cfg(feature = "v2")]
pub struct MerchantIdAndProfileIdAuth {
    pub merchant_id: id_type::MerchantId,
    pub profile_id: id_type::ProfileId,
}

#[cfg(feature = "v2")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for MerchantIdAndProfileIdAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        if state.conf().platform.enabled {
            throw_error_if_platform_merchant_authentication_required(request_headers)?;
        }

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &self.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let profile = state
            .store()
            .find_business_profile_by_merchant_id_profile_id(
                &key_store,
                &self.merchant_id,
                &self.profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;
        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&self.merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let initiator = Some(domain::Initiator::Api {
            merchant_id: merchant.get_id().clone(),
            merchant_account_type: merchant.merchant_account_type,
            publishable_key: merchant.publishable_key.clone(),
        });

        let platform = domain::Platform::new(
            merchant.clone(),
            key_store.clone(),
            merchant.clone(),
            key_store,
            initiator,
        );

        let auth = AuthenticationData {
            platform,
            profile,
            client_secret: None,
        };
        Ok((
            auth.clone(),
            AuthenticationType::MerchantId {
                merchant_id: merchant.get_id().clone(),
            },
        ))
    }
}

#[derive(Debug)]
#[cfg(feature = "v2")]
pub struct PublishableKeyAndProfileIdAuth {
    pub publishable_key: String,
    pub profile_id: id_type::ProfileId,
}

#[async_trait]
#[cfg(feature = "v2")]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for PublishableKeyAndProfileIdAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        _request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let (merchant_account, key_store) = state
            .store()
            .find_merchant_account_by_publishable_key(self.publishable_key.as_str())
            .await
            .map_err(|e| {
                if e.current_context().is_db_not_found() {
                    e.change_context(errors::ApiErrorResponse::Unauthorized)
                } else {
                    e.change_context(errors::ApiErrorResponse::InternalServerError)
                }
            })?;

        let profile = state
            .store()
            .find_business_profile_by_profile_id(&key_store, &self.profile_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
                id: self.profile_id.get_string_repr().to_owned(),
            })?;

        let initiator = Some(domain::Initiator::Api {
            merchant_id: merchant_account.get_id().clone(),
            merchant_account_type: merchant_account.merchant_account_type,
            publishable_key: merchant_account.publishable_key.clone(),
        });

        let platform = domain::Platform::new(
            merchant_account.clone(),
            key_store.clone(),
            merchant_account.clone(),
            key_store,
            initiator,
        );

        Ok((
            AuthenticationData {
                platform,
                profile,
                client_secret: None,
            },
            AuthenticationType::PublishableKey {
                merchant_id: merchant_account.get_id().clone(),
            },
        ))
    }
}

/// Take api-key from `Authorization` header
#[cfg(feature = "v2")]
#[derive(Debug)]
pub struct V2ApiKeyAuth {
    pub allow_connected_scope_operation: bool,
    pub allow_platform_self_operation: bool,
}

#[cfg(feature = "v2")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for V2ApiKeyAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let header_map_struct = HeaderMapStruct::new(request_headers);
        let auth_string = header_map_struct.get_auth_string_from_header()?;

        let api_key = auth_string
            .split(',')
            .find_map(|part| part.trim().strip_prefix("api-key="))
            .ok_or_else(|| {
                report!(errors::ApiErrorResponse::Unauthorized)
                    .attach_printable("Unable to parse api_key")
            })?;
        if api_key.is_empty() {
            return Err(errors::ApiErrorResponse::Unauthorized)
                .attach_printable("API key is empty");
        }

        let profile_id = HeaderMapStruct::new(request_headers)
            .get_id_type_from_header::<id_type::ProfileId>(headers::X_PROFILE_ID)?;

        let api_key = api_keys::PlaintextApiKey::from(api_key);
        let hash_key = {
            let config = state.conf();
            config.api_keys.get_inner().get_hash_key()?
        };
        let hashed_api_key = api_key.keyed_hash(hash_key.peek());

        let stored_api_key = state
            .store()
            .find_api_key_by_hash_optional(hashed_api_key.into())
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError) // If retrieve failed
            .attach_printable("Failed to retrieve API key")?
            .ok_or(report!(errors::ApiErrorResponse::Unauthorized)) // If retrieve returned `None`
            .attach_printable("Merchant not authenticated")?;

        if stored_api_key
            .expires_at
            .map(|expires_at| expires_at < date_time::now())
            .unwrap_or(false)
        {
            return Err(report!(errors::ApiErrorResponse::Unauthorized))
                .attach_printable("API key has expired");
        }

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &stored_api_key.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .change_context(errors::ApiErrorResponse::Unauthorized)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let initiator_merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&stored_api_key.merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        // Validate access based on merchant type and header presence
        check_merchant_access(
            state,
            request_headers,
            initiator_merchant.merchant_account_type,
            self.allow_connected_scope_operation,
            self.allow_platform_self_operation,
        )?;

        let initiator = Some(domain::Initiator::Api {
            merchant_id: initiator_merchant.get_id().clone(),
            merchant_account_type: initiator_merchant.merchant_account_type,
            publishable_key: initiator_merchant.publishable_key.clone(),
        });

        let platform = resolve_platform(
            state,
            request_headers,
            initiator_merchant.clone(),
            key_store,
            initiator,
        )
        .await?;

        let profile = state
            .store()
            .find_business_profile_by_merchant_id_profile_id(
                platform.get_processor().get_key_store(),
                platform.get_processor().get_account().get_id(),
                &profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let auth = AuthenticationData {
            platform,
            profile,
            client_secret: None,
        };
        Ok((
            auth.clone(),
            AuthenticationType::ApiKey {
                merchant_id: initiator_merchant.get_id().clone(),
                key_id: stored_api_key.key_id,
            },
        ))
    }
}

#[cfg(feature = "v2")]
#[derive(Debug)]
pub struct V2ClientAuth(pub common_utils::types::authentication::ResourceId);

#[cfg(feature = "v2")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for V2ClientAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let header_map_struct = HeaderMapStruct::new(request_headers);
        let auth_string = header_map_struct.get_auth_string_from_header()?;

        let publishable_key = auth_string
            .split(',')
            .find_map(|part| part.trim().strip_prefix("publishable-key="))
            .ok_or_else(|| {
                report!(errors::ApiErrorResponse::Unauthorized)
                    .attach_printable("Unable to parse publishable_key")
            })?;

        let client_secret = auth_string
            .split(',')
            .find_map(|part| part.trim().strip_prefix("client-secret="))
            .ok_or_else(|| {
                report!(errors::ApiErrorResponse::Unauthorized)
                    .attach_printable("Unable to parse client_secret")
            })?;

        let db_client_secret: diesel_models::ClientSecretType = state
            .store()
            .get_client_secret(client_secret)
            .await
            .change_context(errors::ApiErrorResponse::Unauthorized)
            .attach_printable("Invalid ephemeral_key")?;

        let profile_id =
            get_id_type_by_key_from_headers(headers::X_PROFILE_ID.to_string(), request_headers)?
                .get_required_value(headers::X_PROFILE_ID)?;

        match (&self.0, &db_client_secret.resource_id) {
            (
                common_utils::types::authentication::ResourceId::Payment(self_id),
                common_utils::types::authentication::ResourceId::Payment(db_id),
            ) => {
                fp_utils::when(self_id != db_id, || {
                    Err::<(), errors::ApiErrorResponse>(errors::ApiErrorResponse::Unauthorized)
                });
            }

            (
                common_utils::types::authentication::ResourceId::Customer(self_id),
                common_utils::types::authentication::ResourceId::Customer(db_id),
            ) => {
                fp_utils::when(self_id != db_id, || {
                    Err::<(), errors::ApiErrorResponse>(errors::ApiErrorResponse::Unauthorized)
                });
            }

            (
                common_utils::types::authentication::ResourceId::PaymentMethodSession(self_id),
                common_utils::types::authentication::ResourceId::PaymentMethodSession(db_id),
            ) => {
                fp_utils::when(self_id != db_id, || {
                    Err::<(), errors::ApiErrorResponse>(errors::ApiErrorResponse::Unauthorized)
                });
            }

            _ => {
                return Err(errors::ApiErrorResponse::Unauthorized.into());
            }
        }

        let (initiator_merchant, key_store) = state
            .store()
            .find_merchant_account_by_publishable_key(publishable_key)
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;
        let merchant_id = initiator_merchant.get_id().clone();

        let initiator = Some(domain::Initiator::Api {
            merchant_id: initiator_merchant.get_id().clone(),
            merchant_account_type: initiator_merchant.merchant_account_type,
            publishable_key: initiator_merchant.publishable_key.clone(),
        });

        let platform = resolve_platform(
            state,
            request_headers,
            initiator_merchant.clone(),
            key_store.clone(),
            initiator,
        )
        .await?;

        if db_client_secret.merchant_id != *platform.get_provider().get_account().get_id() {
            return Err(errors::ApiErrorResponse::Unauthorized.into());
        }

        let profile = state
            .store()
            .find_business_profile_by_merchant_id_profile_id(
                &key_store,
                platform.get_processor().get_account().get_id(),
                &profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let auth = AuthenticationData {
            platform,
            profile,
            client_secret: None,
        };

        Ok((
            auth,
            AuthenticationType::PublishableKey {
                merchant_id: merchant_id.clone(),
            },
        ))
    }
}

#[cfg(feature = "v2")]
pub fn api_or_client_auth<'a, T, A>(
    api_auth: &'a dyn AuthenticateAndFetch<T, A>,
    client_auth: &'a dyn AuthenticateAndFetch<T, A>,
    headers: &HeaderMap,
) -> &'a dyn AuthenticateAndFetch<T, A>
where
{
    if let Ok(val) = HeaderMapStruct::new(headers).get_auth_string_from_header() {
        if val.trim().starts_with("api-key=") {
            api_auth
        } else {
            client_auth
        }
    } else {
        api_auth
    }
}
#[cfg(feature = "v2")]
pub fn api_or_client_or_jwt_auth<'a, T, A>(
    api_auth: &'a dyn AuthenticateAndFetch<T, A>,
    client_auth: &'a dyn AuthenticateAndFetch<T, A>,
    jwt_auth: &'a dyn AuthenticateAndFetch<T, A>,
    headers: &HeaderMap,
) -> &'a dyn AuthenticateAndFetch<T, A>
where
{
    if let Ok(val) = HeaderMapStruct::new(headers).get_auth_string_from_header() {
        if val.trim().starts_with("api-key=") {
            api_auth
        } else if is_jwt_auth(headers) {
            jwt_auth
        } else {
            client_auth
        }
    } else {
        api_auth
    }
}

#[cfg(feature = "v2")]
pub fn sdk_or_api_or_client_auth<'a, T, A>(
    sdk_auth: &'a dyn AuthenticateAndFetch<T, A>,
    api_auth: &'a dyn AuthenticateAndFetch<T, A>,
    client_auth: &'a dyn AuthenticateAndFetch<T, A>,
    headers: &HeaderMap,
) -> &'a dyn AuthenticateAndFetch<T, A>
where
{
    // Check for SDK authorization (base64-encoded)
    if is_sdk_authorization(headers) {
        return sdk_auth;
    }

    api_or_client_auth(api_auth, client_auth, headers)
}

#[cfg(feature = "v2")]
pub fn sdk_or_client_auth<'a, T, A>(
    sdk_auth: &'a dyn AuthenticateAndFetch<T, A>,
    client_auth: &'a dyn AuthenticateAndFetch<T, A>,
    headers: &HeaderMap,
) -> &'a dyn AuthenticateAndFetch<T, A>
where
{
    // Check for SDK authorization (base64-encoded)
    if is_sdk_authorization(headers) {
        return sdk_auth;
    }

    // Fall back to client auth (publishable-key=)
    client_auth
}

#[derive(Debug, Default)]
pub struct PublishableKeyAuth {
    pub allow_connected_scope_operation: bool,
    pub allow_platform_self_operation: bool,
}

#[cfg(feature = "partial-auth")]
impl GetAuthType for PublishableKeyAuth {
    fn get_auth_type(&self) -> detached::PayloadType {
        detached::PayloadType::PublishableKey
    }
}

#[cfg(feature = "partial-auth")]
impl GetMerchantAccessFlags for PublishableKeyAuth {
    fn is_connected_scope_operation_allowed(&self) -> bool {
        self.allow_connected_scope_operation
    }
    fn is_platform_self_operation_allowed(&self) -> bool {
        self.allow_platform_self_operation
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for PublishableKeyAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let publishable_key =
            get_api_key(request_headers).change_context(errors::ApiErrorResponse::Unauthorized)?;

        // Find initiator merchant and key store
        let (initiator_merchant, key_store) = state
            .store()
            .find_merchant_account_by_publishable_key(publishable_key)
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        // Validate access based on merchant type and header presence
        check_merchant_access(
            state,
            request_headers,
            initiator_merchant.merchant_account_type,
            self.allow_connected_scope_operation,
            self.allow_platform_self_operation,
        )?;

        let initiator = Some(domain::Initiator::Api {
            merchant_id: initiator_merchant.get_id().clone(),
            merchant_account_type: initiator_merchant.merchant_account_type,
            publishable_key: initiator_merchant.publishable_key.clone(),
        });

        let platform = resolve_platform(
            state,
            request_headers,
            initiator_merchant.clone(),
            key_store,
            initiator,
        )
        .await?;

        let auth = AuthenticationData {
            platform,
            profile: None,
            client_secret: None,
        };

        Ok((
            auth,
            AuthenticationType::PublishableKey {
                merchant_id: initiator_merchant.get_id().clone(),
            },
        ))
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for PublishableKeyAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let publishable_key =
            get_api_key(request_headers).change_context(errors::ApiErrorResponse::Unauthorized)?;
        let profile_id =
            get_id_type_by_key_from_headers(headers::X_PROFILE_ID.to_string(), request_headers)?
                .get_required_value(headers::X_PROFILE_ID)?;

        // Find initiator merchant and key store
        let (initiator_merchant, key_store) = state
            .store()
            .find_merchant_account_by_publishable_key(publishable_key)
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        // Validate access based on merchant type and header presence
        check_merchant_access(
            state,
            request_headers,
            initiator_merchant.merchant_account_type,
            self.allow_connected_scope_operation,
            self.allow_platform_self_operation,
        )?;

        let initiator = Some(domain::Initiator::Api {
            merchant_id: initiator_merchant.get_id().clone(),
            merchant_account_type: initiator_merchant.merchant_account_type,
            publishable_key: initiator_merchant.publishable_key.clone(),
        });

        let platform = resolve_platform(
            state,
            request_headers,
            initiator_merchant.clone(),
            key_store,
            initiator,
        )
        .await?;

        // Find and validate profile after merchant resolution
        let profile = state
            .store()
            .find_business_profile_by_merchant_id_profile_id(
                platform.get_processor().get_key_store(),
                platform.get_processor().get_account().get_id(),
                &profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let auth = AuthenticationData {
            platform,
            profile,
            client_secret: None,
        };
        Ok((
            auth,
            AuthenticationType::PublishableKey {
                merchant_id: initiator_merchant.get_id().clone(),
            },
        ))
    }
}

/// SDK Authorization authentication using Authorization header
#[cfg(feature = "v1")]
#[derive(Debug, Default)]
pub struct SdkAuthorizationAuth {
    pub allow_connected_scope_operation: bool,
    pub allow_platform_self_operation: bool,
}

#[cfg(feature = "v2")]
#[derive(Debug)]
pub struct SdkAuthorizationAuth {
    pub allow_connected_scope_operation: bool,
    pub allow_platform_self_operation: bool,
    pub resource_id: common_utils::types::authentication::ResourceId,
}

#[cfg(feature = "v1")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for SdkAuthorizationAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        // Get Authorization header
        let sdk_auth_header =
            get_header_value_by_key(headers::AUTHORIZATION.into(), request_headers)?
                .ok_or(errors::ApiErrorResponse::Unauthorized)
                .attach_printable("Missing Authorization header")?;

        // Decode SDK authorization
        let sdk_auth = SdkAuthorization::decode(sdk_auth_header)
            .change_context(errors::ApiErrorResponse::Unauthorized)?;

        // Extract client_secret from decoded SDK authorization
        let client_secret = sdk_auth.client_secret.clone();

        let (initiator_merchant, initiator_merchant_key_store) = match sdk_auth
            .platform_publishable_key
        {
            Some(ref platform_pub_key) => {
                let (platform_merchant, platform_key_store) = state
                    .store()
                    .find_merchant_account_by_publishable_key(platform_pub_key)
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::Unauthorized)
                    .attach_printable("Invalid platform publishable key in SDK authorization")?;

                (platform_merchant, platform_key_store)
            }
            None => {
                let (processor_merchant, processor_key_store) = state
                    .store()
                    .find_merchant_account_by_publishable_key(&sdk_auth.publishable_key)
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::Unauthorized)
                    .attach_printable("Invalid processor publishable key in SDK authorization")?;

                (processor_merchant, processor_key_store)
            }
        };
        let platform = check_sdk_auth_and_resolve_platform(
            state,
            &sdk_auth,
            initiator_merchant.clone(),
            initiator_merchant_key_store,
            self.allow_connected_scope_operation,
            self.allow_platform_self_operation,
        )
        .await?;

        let profile = state
            .store()
            .find_business_profile_by_merchant_id_profile_id(
                platform.get_processor().get_key_store(),
                platform.get_processor().get_account().get_id(),
                &sdk_auth.profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let auth_data = AuthenticationData {
            platform,
            profile: Some(profile),
            client_secret: Some(client_secret),
        };
        Ok((
            auth_data,
            AuthenticationType::SdkAuthorization {
                merchant_id: initiator_merchant.get_id().clone(),
            },
        ))
    }
}

pub async fn check_sdk_auth_and_resolve_platform<A>(
    state: &A,
    sdk_auth: &SdkAuthorization,
    initiator_merchant: domain::MerchantAccount,
    initiator_merchant_key_store: domain::MerchantKeyStore,
    allow_connected_scope_operation: bool,
    allow_platform_self_operation: bool,
) -> RouterResult<domain::Platform>
where
    A: SessionStateInfo + Sync,
{
    let (processor_merchant_account, processor_key_store, platform_account_with_key_store) =
        match initiator_merchant.merchant_account_type {
            MerchantAccountType::Platform => {
                // Check if platform feature is enabled
                state.conf().platform.enabled.then_some(()).ok_or_else(|| {
                    report!(errors::ApiErrorResponse::PlatformAccountAuthNotSupported)
                        .attach_printable("Platform feature is not enabled")
                })?;

                // Look up processor by publishable key from SDK authorization
                let (processor_merchant, processor_key_store) = state
                    .store()
                    .find_merchant_account_by_publishable_key(&sdk_auth.publishable_key)
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::Unauthorized)
                    .attach_printable("Invalid processor publishable key in SDK authorization")?;

                // Validate same organization
                fp_utils::when(
                    processor_merchant.get_org_id() != initiator_merchant.get_org_id(),
                    || {
                        Err(report!(errors::ApiErrorResponse::Unauthorized)).attach_printable(
                            "Platform and processor merchants must be in same organization",
                        )
                    },
                )?;

                // Check authorization based on processor type
                let platform_account = match processor_merchant.merchant_account_type {
                    MerchantAccountType::Connected => {
                        // Platform acting on behalf of connected merchant
                        allow_connected_scope_operation.then_some(()).ok_or_else(|| {
                            report!(errors::ApiErrorResponse::ConnectedAccountAuthNotSupported)
                                .attach_printable(
                                    "Connected merchant scope operation not allowed for this resource",
                                )
                        })?;
                        Some(PlatformAccountWithKeyStore {
                            account: initiator_merchant.clone(),
                            key_store: initiator_merchant_key_store,
                        })
                    }
                    MerchantAccountType::Platform => {
                        // Platform acting on its own resources
                        allow_platform_self_operation.then_some(()).ok_or_else(|| {
                            report!(errors::ApiErrorResponse::Unauthorized).attach_printable(
                                "Platform self operation not allowed for this resource",
                            )
                        })?;
                        None
                    }
                    MerchantAccountType::Standard => {
                        return Err(report!(errors::ApiErrorResponse::Unauthorized))
                            .attach_printable(
                                "Standard merchant type is not valid as processor in platform flow",
                            );
                    }
                };

                (processor_merchant, processor_key_store, platform_account)
            }
            MerchantAccountType::Connected => {
                // Check if platform feature is enabled
                state.conf().platform.enabled.then_some(()).ok_or_else(|| {
                    report!(errors::ApiErrorResponse::ConnectedAccountAuthNotSupported)
                        .attach_printable("Platform feature is not enabled")
                })?;

                // Connected merchant can perform operation if allow_connected_scope_operation is true
                allow_connected_scope_operation
                    .then_some(())
                    .ok_or_else(|| {
                        report!(errors::ApiErrorResponse::ConnectedAccountAuthNotSupported)
                            .attach_printable(
                                "Connected Merchant is not authorized to access the resource",
                            )
                    })?;

                // Connected merchant as initiator
                // Fetch platform merchant and key store using helper function
                let (platform_merchant, platform_key_store) =
                    get_platform_account_and_key_store(state, &initiator_merchant).await?;

                (
                    initiator_merchant.clone(),
                    initiator_merchant_key_store,
                    Some(PlatformAccountWithKeyStore {
                        account: platform_merchant,
                        key_store: platform_key_store,
                    }),
                )
            }
            MerchantAccountType::Standard => {
                // Standard merchant flow
                // Provider and processor are the same merchant
                (
                    initiator_merchant.clone(),
                    initiator_merchant_key_store,
                    None,
                )
            }
        };

    let initiator = Some(domain::Initiator::Api {
        merchant_id: initiator_merchant.get_id().clone(),
        merchant_account_type: initiator_merchant.merchant_account_type,
        publishable_key: initiator_merchant.publishable_key,
    });

    let platform = match platform_account_with_key_store {
        Some(platform_account) => domain::Platform::new(
            platform_account.account,
            platform_account.key_store,
            processor_merchant_account,
            processor_key_store,
            initiator,
        ),
        None => domain::Platform::new(
            processor_merchant_account.clone(),
            processor_key_store.clone(),
            processor_merchant_account,
            processor_key_store,
            initiator,
        ),
    };

    Ok(platform)
}
#[derive(Debug)]
pub(crate) struct JWTAuth {
    pub permission: Permission,
    pub allow_connected: bool,
    pub allow_platform: bool,
}

#[async_trait]
impl<A> AuthenticateAndFetch<(), A> for JWTAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<((), AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }

        authorization::check_tenant(
            payload.tenant_id.clone(),
            &state.session_state().tenant.tenant_id,
        )?;

        let role_info = authorization::get_role_info(state, &payload).await?;
        authorization::check_permission(self.permission, &role_info)?;

        Ok((
            (),
            AuthenticationType::MerchantJwt {
                merchant_id: payload.merchant_id,
                user_id: Some(payload.user_id),
            },
        ))
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for SdkAuthorizationAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        // Get Authorization header
        let sdk_auth_header =
            get_header_value_by_key(headers::AUTHORIZATION.into(), request_headers)?
                .ok_or(errors::ApiErrorResponse::Unauthorized)
                .attach_printable("Missing Authorization header")?;

        // Decode SDK authorization
        let sdk_auth = SdkAuthorization::decode(sdk_auth_header)
            .change_context(errors::ApiErrorResponse::Unauthorized)?;

        // Extract client_secret from decoded SDK authorization
        let client_secret = sdk_auth.client_secret.clone();

        // Validate client_secret against database
        let db_client_secret: diesel_models::ClientSecretType = state
            .store()
            .get_client_secret(&client_secret)
            .await
            .change_context(errors::ApiErrorResponse::Unauthorized)
            .attach_printable("Invalid client_secret in SDK authorization")?;

        let (initiator_merchant, initiator_merchant_key_store) = match sdk_auth
            .platform_publishable_key
        {
            Some(ref platform_pub_key) => {
                let (platform_merchant, platform_key_store) = state
                    .store()
                    .find_merchant_account_by_publishable_key(platform_pub_key)
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::Unauthorized)
                    .attach_printable("Invalid platform publishable key in SDK authorization")?;

                (platform_merchant, platform_key_store)
            }
            None => {
                let (processor_merchant, processor_key_store) = state
                    .store()
                    .find_merchant_account_by_publishable_key(&sdk_auth.publishable_key)
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::Unauthorized)
                    .attach_printable("Invalid processor publishable key in SDK authorization")?;

                (processor_merchant, processor_key_store)
            }
        };

        let platform = check_sdk_auth_and_resolve_platform(
            state,
            &sdk_auth,
            initiator_merchant.clone(),
            initiator_merchant_key_store,
            self.allow_connected_scope_operation,
            self.allow_platform_self_operation,
        )
        .await?;

        if db_client_secret.merchant_id != *platform.get_provider().get_account().get_id() {
            return Err(errors::ApiErrorResponse::Unauthorized.into());
        }

        match (&self.resource_id, &db_client_secret.resource_id) {
            (
                common_utils::types::authentication::ResourceId::Payment(self_id),
                common_utils::types::authentication::ResourceId::Payment(db_id),
            ) => {
                fp_utils::when(self_id != db_id, || {
                    Err::<(), errors::ApiErrorResponse>(errors::ApiErrorResponse::Unauthorized)
                });
            }
            (
                common_utils::types::authentication::ResourceId::Customer(self_id),
                common_utils::types::authentication::ResourceId::Customer(db_id),
            ) => {
                fp_utils::when(self_id != db_id, || {
                    Err::<(), errors::ApiErrorResponse>(errors::ApiErrorResponse::Unauthorized)
                });
            }
            (
                common_utils::types::authentication::ResourceId::PaymentMethodSession(self_id),
                common_utils::types::authentication::ResourceId::PaymentMethodSession(db_id),
            ) => {
                fp_utils::when(self_id != db_id, || {
                    Err::<(), errors::ApiErrorResponse>(errors::ApiErrorResponse::Unauthorized)
                });
            }
            _ => {
                return Err(errors::ApiErrorResponse::Unauthorized.into());
            }
        }

        let profile = state
            .store()
            .find_business_profile_by_merchant_id_profile_id(
                platform.get_processor().get_key_store(),
                platform.get_processor().get_account().get_id(),
                &sdk_auth.profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let auth_data = AuthenticationData {
            platform,
            profile,
            client_secret: Some(client_secret),
        };
        Ok((
            auth_data,
            AuthenticationType::SdkAuthorization {
                merchant_id: initiator_merchant.get_id().clone(),
            },
        ))
    }
}

#[cfg(feature = "olap")]
#[async_trait]
impl<A> AuthenticateAndFetch<UserFromToken, A> for JWTAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(UserFromToken, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }
        authorization::check_tenant(
            payload.tenant_id.clone(),
            &state.session_state().tenant.tenant_id,
        )?;

        let role_info = authorization::get_role_info(state, &payload).await?;
        authorization::check_permission(self.permission, &role_info)?;

        Ok((
            UserFromToken {
                user_id: payload.user_id.clone(),
                merchant_id: payload.merchant_id.clone(),
                org_id: payload.org_id,
                role_id: payload.role_id,
                profile_id: payload.profile_id,
                tenant_id: payload.tenant_id,
            },
            AuthenticationType::MerchantJwt {
                merchant_id: payload.merchant_id,
                user_id: Some(payload.user_id),
            },
        ))
    }
}

#[cfg(feature = "olap")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationDataWithMultipleProfiles, A> for JWTAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationDataWithMultipleProfiles, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }
        authorization::check_tenant(
            payload.tenant_id.clone(),
            &state.session_state().tenant.tenant_id,
        )?;

        let role_info = authorization::get_role_info(state, &payload).await?;
        authorization::check_permission(self.permission, &role_info)?;

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &payload.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .change_context(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&payload.merchant_id, &key_store)
            .await
            .change_context(errors::ApiErrorResponse::InvalidJwtToken)?;

        Ok((
            AuthenticationDataWithMultipleProfiles {
                key_store,
                merchant_account: merchant,
                profile_id_list: None,
            },
            AuthenticationType::MerchantJwt {
                merchant_id: payload.merchant_id,
                user_id: Some(payload.user_id),
            },
        ))
    }
}

pub struct JWTAuthOrganizationFromRoute {
    pub organization_id: id_type::OrganizationId,
    pub required_permission: Permission,
}

#[cfg(feature = "v1")]
#[async_trait]
impl<A> AuthenticateAndFetch<Option<AuthenticationDataWithOrg>, A> for JWTAuthOrganizationFromRoute
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(Option<AuthenticationDataWithOrg>, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }
        authorization::check_tenant(
            payload.tenant_id.clone(),
            &state.session_state().tenant.tenant_id,
        )?;

        let role_info = authorization::get_role_info(state, &payload).await?;
        authorization::check_permission(self.required_permission, &role_info)?;

        // Check if token has access to Organization that has been requested in the route
        if payload.org_id != self.organization_id {
            return Err(report!(errors::ApiErrorResponse::InvalidJwtToken));
        }
        Ok((
            Some(AuthenticationDataWithOrg {
                organization_id: payload.org_id.clone(),
            }),
            AuthenticationType::OrganizationJwt {
                org_id: payload.org_id,
                user_id: payload.user_id,
            },
        ))
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl<A> AuthenticateAndFetch<(), A> for JWTAuthOrganizationFromRoute
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<((), AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }
        authorization::check_tenant(
            payload.tenant_id.clone(),
            &state.session_state().tenant.tenant_id,
        )?;

        let role_info = authorization::get_role_info(state, &payload).await?;
        authorization::check_permission(self.required_permission, &role_info)?;

        // Check if token has access to Organization that has been requested in the route
        if payload.org_id != self.organization_id {
            return Err(report!(errors::ApiErrorResponse::InvalidJwtToken));
        }
        Ok((
            (),
            AuthenticationType::OrganizationJwt {
                org_id: payload.org_id,
                user_id: payload.user_id,
            },
        ))
    }
}

pub struct JWTAuthMerchantFromRoute {
    pub merchant_id: id_type::MerchantId,
    pub required_permission: Permission,
    pub allow_connected: bool,
    pub allow_platform: bool,
}

pub struct JWTAuthMerchantFromHeader {
    pub required_permission: Permission,
    pub allow_connected: bool,
    pub allow_platform: bool,
}

#[async_trait]
impl<A> AuthenticateAndFetch<(), A> for JWTAuthMerchantFromHeader
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<((), AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }
        authorization::check_tenant(
            payload.tenant_id.clone(),
            &state.session_state().tenant.tenant_id,
        )?;

        let role_info = authorization::get_role_info(state, &payload).await?;
        authorization::check_permission(self.required_permission, &role_info)?;

        let merchant_id_from_header = HeaderMapStruct::new(request_headers)
            .get_id_type_from_header::<id_type::MerchantId>(headers::X_MERCHANT_ID)?;

        // Check if token has access to MerchantId that has been requested through headers
        if payload.merchant_id != merchant_id_from_header {
            return Err(report!(errors::ApiErrorResponse::InvalidJwtToken));
        }
        Ok((
            (),
            AuthenticationType::MerchantJwt {
                merchant_id: payload.merchant_id,
                user_id: Some(payload.user_id),
            },
        ))
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for JWTAuthMerchantFromHeader
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }
        authorization::check_tenant(
            payload.tenant_id.clone(),
            &state.session_state().tenant.tenant_id,
        )?;
        let role_info = authorization::get_role_info(state, &payload).await?;
        authorization::check_permission(self.required_permission, &role_info)?;

        let merchant_id_from_header = HeaderMapStruct::new(request_headers)
            .get_id_type_from_header::<id_type::MerchantId>(headers::X_MERCHANT_ID)?;

        // Check if token has access to MerchantId that has been requested through headers
        if payload.merchant_id != merchant_id_from_header {
            return Err(report!(errors::ApiErrorResponse::InvalidJwtToken));
        }

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &payload.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&payload.merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant account for the merchant id")?;

        let profile = state
            .store()
            .find_business_profile_by_merchant_id_profile_id(
                &key_store,
                &payload.merchant_id,
                &payload.profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        check_merchant_access_for_jwt(
            state,
            merchant.merchant_account_type,
            self.allow_connected,
            self.allow_platform,
        )?;

        let initiator = Some(domain::Initiator::Jwt {
            user_id: payload.user_id.clone(),
        });

        let platform = resolve_platform(
            state,
            request_headers,
            merchant.clone(),
            key_store,
            initiator,
        )
        .await?;

        let auth = AuthenticationData {
            platform,
            profile: Some(profile),
            client_secret: None,
        };

        Ok((
            auth,
            AuthenticationType::MerchantJwt {
                merchant_id: payload.merchant_id,
                user_id: Some(payload.user_id),
            },
        ))
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl<A> AuthenticateAndFetch<Option<AuthenticationDataWithOrg>, A> for JWTAuthMerchantFromHeader
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(Option<AuthenticationDataWithOrg>, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }
        authorization::check_tenant(
            payload.tenant_id.clone(),
            &state.session_state().tenant.tenant_id,
        )?;
        let role_info = authorization::get_role_info(state, &payload).await?;
        authorization::check_permission(self.required_permission, &role_info)?;

        let merchant_id_from_header = HeaderMapStruct::new(request_headers)
            .get_id_type_from_header::<id_type::MerchantId>(headers::X_MERCHANT_ID)?;

        // Check if token has access to MerchantId that has been requested through headers
        if payload.merchant_id != merchant_id_from_header {
            return Err(report!(errors::ApiErrorResponse::InvalidJwtToken));
        }

        let auth = Some(AuthenticationDataWithOrg {
            organization_id: payload.org_id,
        });

        Ok((
            auth,
            AuthenticationType::MerchantJwt {
                merchant_id: payload.merchant_id,
                user_id: Some(payload.user_id),
            },
        ))
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for JWTAuthMerchantFromHeader
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }
        let profile_id =
            get_id_type_by_key_from_headers(headers::X_PROFILE_ID.to_string(), request_headers)?
                .get_required_value(headers::X_PROFILE_ID)?;

        let role_info = authorization::get_role_info(state, &payload).await?;
        authorization::check_permission(self.required_permission, &role_info)?;

        let merchant_id_from_header = HeaderMapStruct::new(request_headers)
            .get_id_type_from_header::<id_type::MerchantId>(headers::X_MERCHANT_ID)?;

        // Check if token has access to MerchantId that has been requested through headers
        if payload.merchant_id != merchant_id_from_header {
            return Err(report!(errors::ApiErrorResponse::InvalidJwtToken));
        }

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &payload.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let profile = state
            .store()
            .find_business_profile_by_merchant_id_profile_id(
                &key_store,
                &payload.merchant_id,
                &profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;
        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&payload.merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant account for the merchant id")?;

        check_merchant_access_for_jwt(
            state,
            merchant.merchant_account_type,
            self.allow_connected,
            self.allow_platform,
        )?;

        let initiator = Some(domain::Initiator::Jwt {
            user_id: payload.user_id.clone(),
        });

        let platform = resolve_platform(
            state,
            request_headers,
            merchant.clone(),
            key_store,
            initiator,
        )
        .await?;

        let auth = AuthenticationData {
            platform,
            profile,
            client_secret: None,
        };

        Ok((
            auth,
            AuthenticationType::MerchantJwt {
                merchant_id: payload.merchant_id,
                user_id: Some(payload.user_id),
            },
        ))
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationDataWithoutProfile, A> for JWTAuthMerchantFromHeader
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationDataWithoutProfile, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }

        let role_info = authorization::get_role_info(state, &payload).await?;
        authorization::check_permission(self.required_permission, &role_info)?;

        let merchant_id_from_header = HeaderMapStruct::new(request_headers)
            .get_id_type_from_header::<id_type::MerchantId>(headers::X_MERCHANT_ID)?;

        // Check if token has access to MerchantId that has been requested through headers
        if payload.merchant_id != merchant_id_from_header {
            return Err(report!(errors::ApiErrorResponse::InvalidJwtToken));
        }

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &payload.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&payload.merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant account for the merchant id")?;

        let auth = AuthenticationDataWithoutProfile {
            merchant_account: merchant,
            key_store,
        };

        Ok((
            auth,
            AuthenticationType::MerchantJwt {
                merchant_id: payload.merchant_id,
                user_id: Some(payload.user_id),
            },
        ))
    }
}

#[async_trait]
impl<A> AuthenticateAndFetch<(), A> for JWTAuthMerchantFromRoute
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<((), AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }
        authorization::check_tenant(
            payload.tenant_id.clone(),
            &state.session_state().tenant.tenant_id,
        )?;

        let role_info = authorization::get_role_info(state, &payload).await?;
        authorization::check_permission(self.required_permission, &role_info)?;

        // Check if token has access to MerchantId that has been requested through query param
        if payload.merchant_id != self.merchant_id {
            return Err(report!(errors::ApiErrorResponse::InvalidJwtToken));
        }
        Ok((
            (),
            AuthenticationType::MerchantJwt {
                merchant_id: payload.merchant_id,
                user_id: Some(payload.user_id),
            },
        ))
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for JWTAuthMerchantFromRoute
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }
        authorization::check_tenant(
            payload.tenant_id.clone(),
            &state.session_state().tenant.tenant_id,
        )?;

        if payload.merchant_id != self.merchant_id {
            return Err(report!(errors::ApiErrorResponse::InvalidJwtToken));
        }

        let role_info = authorization::get_role_info(state, &payload).await?;
        authorization::check_permission(self.required_permission, &role_info)?;

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &payload.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&payload.merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant account for the merchant id")?;

        let profile = state
            .store()
            .find_business_profile_by_merchant_id_profile_id(
                &key_store,
                &payload.merchant_id,
                &payload.profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        check_merchant_access_for_jwt(
            state,
            merchant.merchant_account_type,
            self.allow_connected,
            self.allow_platform,
        )?;

        let initiator = Some(domain::Initiator::Jwt {
            user_id: payload.user_id.clone(),
        });

        let platform = resolve_platform(
            state,
            request_headers,
            merchant.clone(),
            key_store,
            initiator,
        )
        .await?;

        let auth = AuthenticationData {
            platform,
            profile: Some(profile),
            client_secret: None,
        };
        Ok((
            auth.clone(),
            AuthenticationType::MerchantJwt {
                merchant_id: payload.merchant_id,
                user_id: Some(payload.user_id),
            },
        ))
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for JWTAuthMerchantFromRoute
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        let profile_id =
            get_id_type_by_key_from_headers(headers::X_PROFILE_ID.to_string(), request_headers)?
                .get_required_value(headers::X_PROFILE_ID)?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }

        if payload.merchant_id != self.merchant_id {
            return Err(report!(errors::ApiErrorResponse::InvalidJwtToken));
        }

        let role_info = authorization::get_role_info(state, &payload).await?;
        authorization::check_permission(self.required_permission, &role_info)?;

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &payload.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;
        let profile = state
            .store()
            .find_business_profile_by_merchant_id_profile_id(
                &key_store,
                &payload.merchant_id,
                &profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;
        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&payload.merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant account for the merchant id")?;

        check_merchant_access_for_jwt(
            state,
            merchant.merchant_account_type,
            self.allow_connected,
            self.allow_platform,
        )?;

        let initiator = Some(domain::Initiator::Jwt {
            user_id: payload.user_id.clone(),
        });

        let platform = resolve_platform(
            state,
            request_headers,
            merchant.clone(),
            key_store,
            initiator,
        )
        .await?;

        let auth = AuthenticationData {
            platform,
            profile,
            client_secret: None,
        };
        Ok((
            auth,
            AuthenticationType::MerchantJwt {
                merchant_id: payload.merchant_id,
                user_id: Some(payload.user_id),
            },
        ))
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationDataWithoutProfile, A> for JWTAuthMerchantFromRoute
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationDataWithoutProfile, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;

        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }

        if payload.merchant_id != self.merchant_id {
            return Err(report!(errors::ApiErrorResponse::InvalidJwtToken));
        }

        let role_info = authorization::get_role_info(state, &payload).await?;
        authorization::check_permission(self.required_permission, &role_info)?;

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &payload.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&payload.merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant account for the merchant id")?;

        let auth = AuthenticationDataWithoutProfile {
            merchant_account: merchant,
            key_store,
        };
        Ok((
            auth.clone(),
            AuthenticationType::MerchantJwt {
                merchant_id: auth.merchant_account.get_id().clone(),
                user_id: Some(payload.user_id),
            },
        ))
    }
}

pub struct JWTAuthMerchantAndProfileFromRoute {
    pub merchant_id: id_type::MerchantId,
    pub profile_id: id_type::ProfileId,
    pub required_permission: Permission,
    pub allow_connected: bool,
    pub allow_platform: bool,
}

#[cfg(feature = "v1")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for JWTAuthMerchantAndProfileFromRoute
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }
        authorization::check_tenant(
            payload.tenant_id.clone(),
            &state.session_state().tenant.tenant_id,
        )?;

        if payload.merchant_id != self.merchant_id {
            return Err(report!(errors::ApiErrorResponse::InvalidJwtToken));
        }

        if payload.profile_id != self.profile_id {
            return Err(report!(errors::ApiErrorResponse::InvalidJwtToken));
        }

        let role_info = authorization::get_role_info(state, &payload).await?;
        authorization::check_permission(self.required_permission, &role_info)?;

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &payload.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&payload.merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant account for the merchant id")?;

        let profile = state
            .store()
            .find_business_profile_by_merchant_id_profile_id(
                &key_store,
                &payload.merchant_id,
                &payload.profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        check_merchant_access_for_jwt(
            state,
            merchant.merchant_account_type,
            self.allow_connected,
            self.allow_platform,
        )?;

        let initiator = Some(domain::Initiator::Jwt {
            user_id: payload.user_id.clone(),
        });

        let platform = resolve_platform(
            state,
            request_headers,
            merchant.clone(),
            key_store,
            initiator,
        )
        .await?;

        let auth = AuthenticationData {
            platform,
            profile: Some(profile),
            client_secret: None,
        };
        Ok((
            auth.clone(),
            AuthenticationType::MerchantJwtWithProfileId {
                merchant_id: payload.merchant_id,
                profile_id: Some(payload.profile_id),
                user_id: payload.user_id,
            },
        ))
    }
}

pub struct JWTAuthProfileFromRoute {
    pub profile_id: id_type::ProfileId,
    pub required_permission: Permission,
    pub allow_connected: bool,
    pub allow_platform: bool,
}

#[cfg(feature = "v1")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for JWTAuthProfileFromRoute
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }
        authorization::check_tenant(
            payload.tenant_id.clone(),
            &state.session_state().tenant.tenant_id,
        )?;

        let role_info = authorization::get_role_info(state, &payload).await?;
        authorization::check_permission(self.required_permission, &role_info)?;

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &payload.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&payload.merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant account for the merchant id")?;

        fp_utils::when(payload.profile_id != self.profile_id, || {
            Err(report!(errors::ApiErrorResponse::InvalidJwtToken))
                .attach_printable("Profile id in JWT does not match profile id in route")
        })?;

        let profile = state
            .store()
            .find_business_profile_by_merchant_id_profile_id(
                &key_store,
                &payload.merchant_id,
                &payload.profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        check_merchant_access_for_jwt(
            state,
            merchant.merchant_account_type,
            self.allow_connected,
            self.allow_platform,
        )?;

        let initiator = Some(domain::Initiator::Jwt {
            user_id: payload.user_id.clone(),
        });

        // If both of them are same then proceed with the profile id present in the request
        let platform = resolve_platform(
            state,
            request_headers,
            merchant.clone(),
            key_store,
            initiator,
        )
        .await?;

        let auth = AuthenticationData {
            platform,
            profile: Some(profile),
            client_secret: None,
        };
        Ok((
            auth.clone(),
            AuthenticationType::MerchantJwt {
                merchant_id: payload.merchant_id,
                user_id: Some(payload.user_id),
            },
        ))
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for JWTAuthProfileFromRoute
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }
        let profile_id =
            get_id_type_by_key_from_headers(headers::X_PROFILE_ID.to_string(), request_headers)?
                .get_required_value(headers::X_PROFILE_ID)?;

        let role_info = authorization::get_role_info(state, &payload).await?;
        authorization::check_permission(self.required_permission, &role_info)?;

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &payload.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let profile = state
            .store()
            .find_business_profile_by_merchant_id_profile_id(
                &key_store,
                &payload.merchant_id,
                &profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;
        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&payload.merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant account for the merchant id")?;

        check_merchant_access_for_jwt(
            state,
            merchant.merchant_account_type,
            self.allow_connected,
            self.allow_platform,
        )?;

        let initiator = Some(domain::Initiator::Jwt {
            user_id: payload.user_id.clone(),
        });

        let platform = resolve_platform(
            state,
            request_headers,
            merchant.clone(),
            key_store,
            initiator,
        )
        .await?;

        let auth = AuthenticationData {
            platform,
            profile,
            client_secret: None,
        };
        Ok((
            auth.clone(),
            AuthenticationType::MerchantJwt {
                merchant_id: payload.merchant_id,
                user_id: Some(payload.user_id),
            },
        ))
    }
}

pub async fn parse_jwt_payload<A, T>(headers: &HeaderMap, state: &A) -> RouterResult<T>
where
    T: serde::de::DeserializeOwned,
    A: SessionStateInfo + Sync,
{
    let cookie_token_result =
        get_cookie_from_header(headers).and_then(cookies::get_jwt_from_cookies);
    let auth_header_token_result = get_jwt_from_authorization_header(headers);
    let force_cookie = state.conf().user.force_cookies;

    logger::info!(
        user_agent = ?headers.get(headers::USER_AGENT),
        header_names = ?headers.keys().collect::<Vec<_>>(),
        is_token_equal =
            auth_header_token_result.as_deref().ok() == cookie_token_result.as_deref().ok(),
        cookie_error = ?cookie_token_result.as_ref().err(),
        token_error = ?auth_header_token_result.as_ref().err(),
        force_cookie,
    );

    let final_token = if force_cookie {
        cookie_token_result?
    } else {
        auth_header_token_result?.to_owned()
    };

    decode_jwt(&final_token, state).await
}

#[cfg(feature = "v1")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for JWTAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }
        authorization::check_tenant(
            payload.tenant_id.clone(),
            &state.session_state().tenant.tenant_id,
        )?;

        let role_info = authorization::get_role_info(state, &payload).await?;
        authorization::check_permission(self.permission, &role_info)?;

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &payload.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&payload.merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant account for the merchant id")?;

        let profile = state
            .store()
            .find_business_profile_by_merchant_id_profile_id(
                &key_store,
                &payload.merchant_id,
                &payload.profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        check_merchant_access_for_jwt(
            state,
            merchant.merchant_account_type,
            self.allow_connected,
            self.allow_platform,
        )?;

        let initiator = Some(domain::Initiator::Jwt {
            user_id: payload.user_id.clone(),
        });

        let platform = resolve_platform(
            state,
            request_headers,
            merchant.clone(),
            key_store,
            initiator,
        )
        .await?;

        let auth = AuthenticationData {
            platform,
            profile: Some(profile),
            client_secret: None,
        };
        Ok((
            auth,
            AuthenticationType::MerchantJwt {
                merchant_id: payload.merchant_id,
                user_id: Some(payload.user_id),
            },
        ))
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for JWTAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }
        authorization::check_tenant(
            payload.tenant_id.clone(),
            &state.session_state().tenant.tenant_id,
        )?;

        let profile_id = HeaderMapStruct::new(request_headers)
            .get_id_type_from_header::<id_type::ProfileId>(headers::X_PROFILE_ID)?;

        let role_info = authorization::get_role_info(state, &payload).await?;
        authorization::check_permission(self.permission, &role_info)?;

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &payload.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let profile = state
            .store()
            .find_business_profile_by_merchant_id_profile_id(
                &key_store,
                &payload.merchant_id,
                &profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&payload.merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant account for the merchant id")?;

        check_merchant_access_for_jwt(
            state,
            merchant.merchant_account_type,
            self.allow_connected,
            self.allow_platform,
        )?;

        let initiator = Some(domain::Initiator::Jwt {
            user_id: payload.user_id.clone(),
        });

        let platform = resolve_platform(
            state,
            request_headers,
            merchant.clone(),
            key_store,
            initiator,
        )
        .await?;

        let auth = AuthenticationData {
            platform,
            profile,
            client_secret: None,
        };
        Ok((
            auth,
            AuthenticationType::MerchantJwt {
                merchant_id: payload.merchant_id,
                user_id: Some(payload.user_id),
            },
        ))
    }
}

pub type AuthenticationDataWithUserId = (AuthenticationData, Option<String>);

#[cfg(feature = "v1")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationDataWithUserId, A> for JWTAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationDataWithUserId, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }
        authorization::check_tenant(
            payload.tenant_id.clone(),
            &state.session_state().tenant.tenant_id,
        )?;

        let role_info = authorization::get_role_info(state, &payload).await?;
        authorization::check_permission(self.permission, &role_info)?;

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &payload.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .change_context(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&payload.merchant_id, &key_store)
            .await
            .change_context(errors::ApiErrorResponse::InvalidJwtToken)?;

        let profile = state
            .store()
            .find_business_profile_by_merchant_id_profile_id(
                &key_store,
                &payload.merchant_id,
                &payload.profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        check_merchant_access_for_jwt(
            state,
            merchant.merchant_account_type,
            self.allow_connected,
            self.allow_platform,
        )?;

        let initiator = Some(domain::Initiator::Jwt {
            user_id: payload.user_id.clone(),
        });

        let platform = resolve_platform(
            state,
            request_headers,
            merchant.clone(),
            key_store,
            initiator,
        )
        .await?;
        let auth = AuthenticationData {
            platform,
            profile: Some(profile),
            client_secret: None,
        };
        Ok((
            (auth.clone(), Some(payload.user_id.clone())),
            AuthenticationType::MerchantJwt {
                merchant_id: payload.merchant_id,
                user_id: None,
            },
        ))
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationDataWithUserId, A> for JWTAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationDataWithUserId, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }
        authorization::check_tenant(
            payload.tenant_id.clone(),
            &state.session_state().tenant.tenant_id,
        )?;

        let role_info = authorization::get_role_info(state, &payload).await?;
        authorization::check_permission(self.permission, &role_info)?;

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &payload.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .change_context(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&payload.merchant_id, &key_store)
            .await
            .change_context(errors::ApiErrorResponse::InvalidJwtToken)?;

        let profile = state
            .store()
            .find_business_profile_by_merchant_id_profile_id(
                &key_store,
                &payload.merchant_id,
                &payload.profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let initiator = Some(domain::Initiator::Jwt {
            user_id: payload.user_id.clone(),
        });

        let platform = resolve_platform(
            state,
            request_headers,
            merchant.clone(),
            key_store,
            initiator,
        )
        .await?;

        let auth = AuthenticationData {
            platform,
            profile,
            client_secret: None,
        };

        Ok((
            (auth.clone(), Some(payload.user_id.clone())),
            AuthenticationType::MerchantJwt {
                merchant_id: payload.merchant_id,
                user_id: Some(payload.user_id),
            },
        ))
    }
}

pub struct DashboardNoPermissionAuth {
    pub allow_connected: bool,
    pub allow_platform: bool,
}

#[cfg(feature = "olap")]
#[async_trait]
impl<A> AuthenticateAndFetch<UserFromToken, A> for DashboardNoPermissionAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(UserFromToken, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }

        authorization::check_tenant(
            payload.tenant_id.clone(),
            &state.session_state().tenant.tenant_id,
        )?;

        Ok((
            UserFromToken {
                user_id: payload.user_id.clone(),
                merchant_id: payload.merchant_id.clone(),
                org_id: payload.org_id,
                role_id: payload.role_id,
                profile_id: payload.profile_id,
                tenant_id: payload.tenant_id,
            },
            AuthenticationType::MerchantJwt {
                merchant_id: payload.merchant_id,
                user_id: Some(payload.user_id),
            },
        ))
    }
}

#[cfg(feature = "olap")]
#[async_trait]
impl<A> AuthenticateAndFetch<(), A> for DashboardNoPermissionAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<((), AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }
        authorization::check_tenant(
            payload.tenant_id.clone(),
            &state.session_state().tenant.tenant_id,
        )?;

        Ok(((), AuthenticationType::NoAuth))
    }
}

#[cfg(feature = "olap")]
#[async_trait]
impl<A> AuthenticateAndFetch<Option<UserFromToken>, A> for DashboardNoPermissionAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(Option<UserFromToken>, AuthenticationType)> {
        <Self as AuthenticateAndFetch<UserFromToken, A>>::authenticate_and_fetch(
            self,
            request_headers,
            state,
        )
        .await
        .map(|(user, auth_type)| (Some(user), auth_type))
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for DashboardNoPermissionAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        authorization::check_tenant(
            payload.tenant_id.clone(),
            &state.session_state().tenant.tenant_id,
        )?;

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &payload.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .change_context(errors::ApiErrorResponse::Unauthorized)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&payload.merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let profile = state
            .store()
            .find_business_profile_by_merchant_id_profile_id(
                &key_store,
                &payload.merchant_id,
                &payload.profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        check_merchant_access_for_jwt(
            state,
            merchant.merchant_account_type,
            self.allow_connected,
            self.allow_platform,
        )?;

        let initiator = Some(domain::Initiator::Jwt {
            user_id: payload.user_id.clone(),
        });

        let platform = resolve_platform(
            state,
            request_headers,
            merchant.clone(),
            key_store,
            initiator,
        )
        .await?;

        let auth = AuthenticationData {
            platform,
            profile: Some(profile),
            client_secret: None,
        };
        Ok((
            auth,
            AuthenticationType::MerchantJwt {
                merchant_id: payload.merchant_id,
                user_id: Some(payload.user_id),
            },
        ))
    }
}

pub struct JWTAndEmbeddedAuth {
    pub merchant_id_from_route: Option<id_type::MerchantId>,
    pub permission: Option<Permission>,
    pub allow_connected: bool,
    pub allow_platform: bool,
}

#[cfg(feature = "v1")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for JWTAndEmbeddedAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthOrEmbeddedClaims>(request_headers, state).await?;

        if let AuthOrEmbeddedClaims::AuthToken(ref auth_payload) = payload {
            if auth_payload.check_in_blacklist(state).await? {
                return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
            }
            if let Some(required_permission) = self.permission {
                let role_info = authorization::get_role_info(state, auth_payload).await?;
                authorization::check_permission(required_permission, &role_info)?;
            }
        }

        authorization::check_tenant(
            payload.get_tenant_id().cloned(),
            &state.session_state().tenant.tenant_id,
        )?;

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                payload.get_merchant_id(),
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(payload.get_merchant_id(), &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant account for the merchant id")?;

        let profile = state
            .store()
            .find_business_profile_by_merchant_id_profile_id(
                &key_store,
                payload.get_merchant_id(),
                payload.get_profile_id(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch business profile")?;

        fp_utils::when(
            self.merchant_id_from_route
                .as_ref()
                .is_some_and(|mid_from_route| payload.get_merchant_id() != mid_from_route),
            || Err(report!(errors::ApiErrorResponse::InvalidJwtToken)),
        )?;

        check_merchant_access_for_jwt(
            state,
            merchant.merchant_account_type,
            self.allow_connected,
            self.allow_platform,
        )?;

        let initiator = if let AuthOrEmbeddedClaims::AuthToken(ref auth_payload) = payload {
            Some(domain::Initiator::Jwt {
                user_id: auth_payload.user_id.clone(),
            })
        } else {
            Some(domain::Initiator::EmbeddedToken {
                merchant_id: payload.get_merchant_id().clone(),
            })
        };

        let platform = resolve_platform(
            state,
            request_headers,
            merchant.clone(),
            key_store,
            initiator,
        )
        .await?;

        let auth = AuthenticationData {
            platform,
            profile: Some(profile),
            client_secret: None,
        };
        let auth_type = match payload {
            AuthOrEmbeddedClaims::AuthToken(auth_payload) => AuthenticationType::MerchantJwt {
                merchant_id: auth_payload.merchant_id,
                user_id: Some(auth_payload.user_id),
            },
            AuthOrEmbeddedClaims::EmbeddedToken(embedded_payload) => {
                AuthenticationType::EmbeddedJwt {
                    merchant_id: embedded_payload.merchant_id,
                    profile_id: embedded_payload.profile_id,
                }
            }
        };
        Ok((auth, auth_type))
    }
}

pub trait ClientSecretFetch {
    fn get_client_secret(&self) -> Option<&String>;
}
#[cfg(feature = "payouts")]
impl ClientSecretFetch for payouts::PayoutCreateRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref()
    }
}

#[cfg(feature = "v1")]
impl ClientSecretFetch for payments::PaymentsRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref()
    }
}

#[cfg(feature = "v1")]
impl ClientSecretFetch for payments::PaymentsEligibilityRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret
            .as_ref()
            .map(|client_secret| client_secret.peek())
    }
}

#[cfg(feature = "v1")]
impl ClientSecretFetch for api_models::blocklist::ListBlocklistQuery {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref()
    }
}

#[cfg(feature = "v1")]
impl ClientSecretFetch for payments::PaymentsRetrieveRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref()
    }
}

#[cfg(feature = "v1")]
impl ClientSecretFetch for PaymentMethodListRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref()
    }
}

impl ClientSecretFetch for payments::PaymentsPostSessionTokensRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret
            .as_ref()
            .map(|client_secret| client_secret.peek())
    }
}

impl ClientSecretFetch for payments::PaymentsDynamicTaxCalculationRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret
            .as_ref()
            .map(|client_secret| client_secret.peek())
    }
}

impl ClientSecretFetch for payments::PaymentsExternalAuthenticationRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret
            .as_ref()
            .map(|client_secret| client_secret.peek())
    }
}

#[cfg(feature = "v1")]
impl ClientSecretFetch for PaymentMethodCreate {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref()
    }
}

impl ClientSecretFetch for api_models::cards_info::CardsInfoRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref()
    }
}

impl ClientSecretFetch for payments::RetrievePaymentLinkRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref()
    }
}

impl ClientSecretFetch for api_models::pm_auth::LinkTokenCreateRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref()
    }
}

impl ClientSecretFetch for api_models::pm_auth::ExchangeTokenCreateRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref()
    }
}

#[cfg(feature = "v1")]
impl ClientSecretFetch for api_models::payment_methods::PaymentMethodUpdate {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref()
    }
}

#[cfg(feature = "v1")]
impl ClientSecretFetch for api_models::subscription::ConfirmSubscriptionRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref().map(|s| s.as_string())
    }
}

#[cfg(feature = "v1")]
impl ClientSecretFetch for api_models::subscription::GetSubscriptionItemsQuery {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref().map(|s| s.as_string())
    }
}

#[cfg(feature = "v1")]
impl ClientSecretFetch for api_models::authentication::AuthenticationEligibilityRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret
            .as_ref()
            .map(|client_secret| client_secret.peek())
    }
}

impl ClientSecretFetch for api_models::authentication::AuthenticationAuthenticateRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret
            .as_ref()
            .map(|client_secret| client_secret.peek())
    }
}

#[cfg(feature = "v1")]
impl ClientSecretFetch for api_models::authentication::AuthenticationEligibilityCheckRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret
            .as_ref()
            .map(|client_secret| client_secret.peek())
    }
}

impl ClientSecretFetch for api_models::authentication::AuthenticationSyncRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret
            .as_ref()
            .map(|client_secret| client_secret.peek())
    }
}

impl ClientSecretFetch for api_models::authentication::AuthenticationSessionTokenRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret
            .as_ref()
            .map(|client_secret| client_secret.peek())
    }
}

pub fn get_auth_type_and_flow<A: SessionStateInfo + Sync + Send>(
    headers: &HeaderMap,
    api_auth: ApiKeyAuth,
) -> RouterResult<(
    Box<dyn AuthenticateAndFetch<AuthenticationData, A>>,
    api::AuthFlow,
)> {
    let api_key = get_api_key(headers)?;

    if api_key.starts_with("pk_") {
        return Ok((
            Box::new(HeaderAuth(PublishableKeyAuth {
                allow_connected_scope_operation: api_auth.allow_connected_scope_operation,
                allow_platform_self_operation: api_auth.allow_platform_self_operation,
            })),
            api::AuthFlow::Client,
        ));
    }
    Ok((Box::new(HeaderAuth(api_auth)), api::AuthFlow::Merchant))
}

#[cfg(feature = "v1")]
/// Wrapper function to check Authorization header and call get_auth_type_and_flow if not present
pub fn check_authorization_header_or_get_auth<A: SessionStateInfo + Sync + Send>(
    headers: &HeaderMap,
    api_auth: ApiKeyAuth,
) -> RouterResult<(
    Box<dyn AuthenticateAndFetch<AuthenticationData, A>>,
    api::AuthFlow,
)> {
    match get_header_value_by_key(headers::AUTHORIZATION.into(), headers)? {
        // If Authorization header is present, use SdkAuthorizationAuth
        Some(_) => Ok((
            Box::new(SdkAuthorizationAuth {
                allow_connected_scope_operation: api_auth.allow_connected_scope_operation,
                allow_platform_self_operation: api_auth.allow_platform_self_operation,
            }),
            api::AuthFlow::Client,
        )),
        // If Authorization header is not present, use existing flow
        None => get_auth_type_and_flow(headers, api_auth),
    }
}

pub fn check_client_secret_and_get_auth<T>(
    headers: &HeaderMap,
    payload: &impl ClientSecretFetch,
    api_auth: ApiKeyAuth,
) -> RouterResult<(
    Box<dyn AuthenticateAndFetch<AuthenticationData, T>>,
    api::AuthFlow,
)>
where
    T: SessionStateInfo + Sync + Send,
    ApiKeyAuth: AuthenticateAndFetch<AuthenticationData, T>,
    PublishableKeyAuth: AuthenticateAndFetch<AuthenticationData, T>,
{
    let api_key = get_api_key(headers)?;
    if api_key.starts_with("pk_") {
        payload
            .get_client_secret()
            .check_value_present("client_secret")
            .map_err(|_| errors::ApiErrorResponse::MissingRequiredField {
                field_name: "client_secret",
            })?;
        return Ok((
            Box::new(HeaderAuth(PublishableKeyAuth {
                allow_connected_scope_operation: api_auth.allow_connected_scope_operation,
                allow_platform_self_operation: api_auth.allow_platform_self_operation,
            })),
            api::AuthFlow::Client,
        ));
    }

    if payload.get_client_secret().is_some() {
        return Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "client_secret is not a valid parameter".to_owned(),
        }
        .into());
    }
    Ok((Box::new(HeaderAuth(api_auth)), api::AuthFlow::Merchant))
}

/// Checks Authorization header first for SDK auth, if not exists calls check_client_secret_and_get_auth
#[cfg(feature = "v1")]
pub fn check_sdk_auth_and_get_auth<T>(
    headers: &HeaderMap,
    payload: &impl ClientSecretFetch,
    api_auth: ApiKeyAuth,
) -> RouterResult<(
    Box<dyn AuthenticateAndFetch<AuthenticationData, T>>,
    api::AuthFlow,
)>
where
    T: SessionStateInfo + Sync + Send,
    ApiKeyAuth: AuthenticateAndFetch<AuthenticationData, T>,
    PublishableKeyAuth: AuthenticateAndFetch<AuthenticationData, T>,
    SdkAuthorizationAuth: AuthenticateAndFetch<AuthenticationData, T>,
{
    // Check Authorization header first
    match get_header_value_by_key(headers::AUTHORIZATION.into(), headers)? {
        // SDK authorization flow
        Some(_auth) => Ok((
            Box::new(SdkAuthorizationAuth {
                allow_connected_scope_operation: api_auth.allow_connected_scope_operation,
                allow_platform_self_operation: api_auth.allow_platform_self_operation,
            }),
            api::AuthFlow::Client,
        )),
        None => {
            // Use existing client_secret and publishable key check
            check_client_secret_and_get_auth(headers, payload, api_auth)
        }
    }
}

pub async fn get_ephemeral_or_other_auth<T>(
    headers: &HeaderMap,
    is_merchant_flow: bool,
    payload: Option<&impl ClientSecretFetch>,
    api_auth: ApiKeyAuth,
) -> RouterResult<(
    Box<dyn AuthenticateAndFetch<AuthenticationData, T>>,
    api::AuthFlow,
    bool,
)>
where
    T: SessionStateInfo + Sync + Send,
    ApiKeyAuth: AuthenticateAndFetch<AuthenticationData, T>,
    PublishableKeyAuth: AuthenticateAndFetch<AuthenticationData, T>,
    EphemeralKeyAuth: AuthenticateAndFetch<AuthenticationData, T>,
{
    let api_key = get_api_key(headers)?;

    if api_key.starts_with("epk") {
        Ok((Box::new(EphemeralKeyAuth), api::AuthFlow::Client, true))
    } else if is_merchant_flow {
        Ok((
            Box::new(HeaderAuth(api_auth)),
            api::AuthFlow::Merchant,
            false,
        ))
    } else {
        let payload = payload.get_required_value("ClientSecretFetch")?;
        let (auth, auth_flow) = check_client_secret_and_get_auth(headers, payload, api_auth)?;
        Ok((auth, auth_flow, false))
    }
}

#[cfg(feature = "v1")]
pub fn is_ephemeral_auth<A: SessionStateInfo + Sync + Send>(
    headers: &HeaderMap,
    api_auth: ApiKeyAuth,
) -> RouterResult<Box<dyn AuthenticateAndFetch<AuthenticationData, A>>> {
    let api_key = get_api_key(headers)?;

    if !api_key.starts_with("epk") {
        Ok(Box::new(HeaderAuth(api_auth)))
    } else {
        Ok(Box::new(EphemeralKeyAuth))
    }
}

pub fn is_jwt_auth(headers: &HeaderMap) -> bool {
    let header_map_struct = HeaderMapStruct::new(headers);
    match header_map_struct.get_auth_string_from_header() {
        Ok(auth_str) => auth_str.starts_with("Bearer"),
        Err(_) => get_cookie_from_header(headers)
            .and_then(cookies::get_jwt_from_cookies)
            .is_ok(),
    }
}

/// Checks if Authorization header contains SDK authorization (base64-encoded)
#[cfg(feature = "v2")]
pub fn is_sdk_authorization(headers: &HeaderMap) -> bool {
    if let Ok(auth_val) = HeaderMapStruct::new(headers).get_auth_string_from_header() {
        let trimmed = auth_val.trim();
        // Try to decode using SdkAuthorization::decode - if it succeeds, it's SDK auth
        return SdkAuthorization::decode(trimmed).is_ok();
    }
    false
}

pub fn is_internal_api_key_merchant_id_profile_id_auth(
    headers: &HeaderMap,
    internal_api_key_auth: settings::InternalMerchantIdProfileIdAuthSettings,
) -> bool {
    internal_api_key_auth.enabled
        && headers.contains_key(headers::X_INTERNAL_API_KEY)
        && headers.contains_key(headers::X_MERCHANT_ID)
        && headers.contains_key(headers::X_PROFILE_ID)
}

#[cfg(feature = "v1")]
pub fn check_internal_api_key_auth<T>(
    headers: &HeaderMap,
    payload: &impl ClientSecretFetch,
    api_auth: ApiKeyAuth,
    internal_api_key_auth: settings::InternalMerchantIdProfileIdAuthSettings,
) -> RouterResult<(
    Box<dyn AuthenticateAndFetch<AuthenticationData, T>>,
    api::AuthFlow,
)>
where
    T: SessionStateInfo + Sync + Send,
    ApiKeyAuth: AuthenticateAndFetch<AuthenticationData, T>,
{
    if is_internal_api_key_merchant_id_profile_id_auth(headers, internal_api_key_auth) {
        Ok((
            // HeaderAuth(api_auth) will never be called in this case as the internal auth will be checked first
            Box::new(InternalMerchantIdProfileIdAuth(HeaderAuth(api_auth))),
            api::AuthFlow::Merchant,
        ))
    } else {
        check_sdk_auth_and_get_auth(headers, payload, api_auth)
    }
}

#[cfg(feature = "v1")]
pub fn check_internal_api_key_auth_no_client_secret<T>(
    headers: &HeaderMap,
    api_auth: ApiKeyAuth,
    internal_api_key_auth: settings::InternalMerchantIdProfileIdAuthSettings,
) -> RouterResult<(
    Box<dyn AuthenticateAndFetch<AuthenticationData, T>>,
    api::AuthFlow,
)>
where
    T: SessionStateInfo + Sync + Send,
    ApiKeyAuth: AuthenticateAndFetch<AuthenticationData, T>,
{
    if is_internal_api_key_merchant_id_profile_id_auth(headers, internal_api_key_auth) {
        Ok((
            // HeaderAuth(api_auth) will never be called in this case as the internal auth will be checked first
            Box::new(InternalMerchantIdProfileIdAuth(HeaderAuth(api_auth))),
            api::AuthFlow::Merchant,
        ))
    } else {
        let (auth, auth_flow) = check_authorization_header_or_get_auth(headers, api_auth)?;
        Ok((auth, auth_flow))
    }
}

#[cfg(feature = "v2")]
pub fn check_internal_api_key_auth_no_client_secret<T>(
    headers: &HeaderMap,
    api_auth: V2ApiKeyAuth,
    internal_api_key_auth: settings::InternalMerchantIdProfileIdAuthSettings,
) -> RouterResult<(
    Box<dyn AuthenticateAndFetch<AuthenticationData, T>>,
    common_enums::ApiKeyType,
)>
where
    T: SessionStateInfo + Sync + Send,
    ApiKeyAuth: AuthenticateAndFetch<AuthenticationData, T>,
{
    if is_internal_api_key_merchant_id_profile_id_auth(headers, internal_api_key_auth) {
        Ok((
            // HeaderAuth(api_auth) will never be called in this case as the internal auth will be checked first
            Box::new(InternalMerchantIdProfileIdAuth(HeaderAuth(api_auth))),
            common_enums::ApiKeyType::Internal,
        ))
    } else {
        Ok((
            Box::new(HeaderAuth(api_auth)),
            common_enums::ApiKeyType::External,
        ))
    }
}

#[cfg(feature = "v2")]
pub(crate) fn check_internal_api_key_or_dashboard_auth_no_client_secret<T>(
    headers: &HeaderMap,
    api_auth: V2ApiKeyAuth,
    jwt_auth: JWTAuth,
    internal_api_key_auth: settings::InternalMerchantIdProfileIdAuthSettings,
) -> RouterResult<(
    Box<dyn AuthenticateAndFetch<AuthenticationData, T>>,
    common_enums::ApiKeyType,
)>
where
    T: SessionStateInfo + Sync + Send,
    ApiKeyAuth: AuthenticateAndFetch<AuthenticationData, T>,
{
    if is_internal_api_key_merchant_id_profile_id_auth(headers, internal_api_key_auth) {
        Ok((
            // HeaderAuth(api_auth) will never be called in this case as the internal auth will be checked first
            Box::new(InternalMerchantIdProfileIdAuth(HeaderAuth(api_auth))),
            common_enums::ApiKeyType::Internal,
        ))
    } else if is_jwt_auth(headers) {
        Ok((Box::new(jwt_auth), common_enums::ApiKeyType::External))
    } else {
        Ok((
            Box::new(HeaderAuth(api_auth)),
            common_enums::ApiKeyType::External,
        ))
    }
}

pub async fn decode_jwt<T>(token: &str, state: &impl SessionStateInfo) -> RouterResult<T>
where
    T: serde::de::DeserializeOwned,
{
    let conf = state.conf();
    let secret = conf.secrets.get_inner().jwt_secret.peek().as_bytes();

    let key = DecodingKey::from_secret(secret);
    decode::<T>(token, &key, &Validation::new(Algorithm::HS256))
        .map(|decoded| decoded.claims)
        .map_err(|e| {
            if e.kind() == &ExpiredSignature {
                report!(errors::ApiErrorResponse::ExpiredJwtToken)
            } else {
                report!(errors::ApiErrorResponse::InvalidJwtToken)
            }
        })
}

pub fn get_api_key(headers: &HeaderMap) -> RouterResult<&str> {
    get_header_value_by_key("api-key".into(), headers)?.get_required_value("api_key")
}

pub fn get_header_value_by_key(key: String, headers: &HeaderMap) -> RouterResult<Option<&str>> {
    headers
        .get(&key)
        .map(|source_str| {
            source_str
                .to_str()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(format!(
                    "Failed to convert header value to string for header key: {key}",
                ))
        })
        .transpose()
}
pub fn get_id_type_by_key_from_headers<T: FromStr>(
    key: String,
    headers: &HeaderMap,
) -> RouterResult<Option<T>> {
    get_header_value_by_key(key.clone(), headers)?
        .map(|str_value| T::from_str(str_value))
        .transpose()
        .map_err(|_err| {
            error_stack::report!(errors::ApiErrorResponse::InvalidDataFormat {
                field_name: key,
                expected_format: "Valid Id String".to_string(),
            })
        })
}

pub fn get_jwt_from_authorization_header(headers: &HeaderMap) -> RouterResult<&str> {
    headers
        .get(headers::AUTHORIZATION)
        .get_required_value(headers::AUTHORIZATION)?
        .to_str()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to convert JWT token to string")?
        .strip_prefix("Bearer ")
        .ok_or(errors::ApiErrorResponse::InvalidJwtToken.into())
}

pub fn get_cookie_from_header(headers: &HeaderMap) -> RouterResult<&str> {
    let cookie = headers
        .get(cookies::get_cookie_header())
        .ok_or(report!(errors::ApiErrorResponse::CookieNotFound))?;

    cookie
        .to_str()
        .change_context(errors::ApiErrorResponse::InvalidCookie)
}

pub fn strip_jwt_token(token: &str) -> RouterResult<&str> {
    token
        .strip_prefix("Bearer ")
        .ok_or_else(|| errors::ApiErrorResponse::InvalidJwtToken.into())
}

pub fn strip_basic_auth_token(token: &str) -> RouterResult<&str> {
    token
        .strip_prefix("Basic ")
        .ok_or_else(|| errors::ApiErrorResponse::InvalidBasicAuth.into())
}

fn parse_basic_auth_credentials(
    headers: &HeaderMap,
) -> RouterResult<(String, masking::Secret<String>)> {
    let authorization_header = get_header_value_by_key(headers::AUTHORIZATION.to_string(), headers)
        .change_context(errors::ApiErrorResponse::InvalidBasicAuth)?
        .get_required_value(headers::AUTHORIZATION)?;

    let encoded_credentials = strip_basic_auth_token(authorization_header)?;

    let decoded_bytes = BASE64_ENGINE
        .decode(encoded_credentials)
        .change_context(errors::ApiErrorResponse::InvalidBasicAuth)?;

    let credential_string = String::from_utf8(decoded_bytes)
        .change_context(errors::ApiErrorResponse::InvalidBasicAuth)?;

    let (identifier, secret) = credential_string
        .split_once(':')
        .ok_or(errors::ApiErrorResponse::InvalidBasicAuth)?;

    let identifier = identifier.trim();
    let secret = secret.trim();

    if identifier.is_empty() || secret.is_empty() {
        return Err(errors::ApiErrorResponse::InvalidBasicAuth.into());
    }

    Ok((
        identifier.to_string(),
        masking::Secret::new(secret.to_string()),
    ))
}

pub fn auth_type<'a, T, A>(
    default_auth: &'a dyn AuthenticateAndFetch<T, A>,
    jwt_auth_type: &'a dyn AuthenticateAndFetch<T, A>,
    headers: &HeaderMap,
) -> &'a dyn AuthenticateAndFetch<T, A>
where
{
    if is_jwt_auth(headers) {
        return jwt_auth_type;
    }
    default_auth
}

#[cfg(feature = "recon")]
#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationDataWithUser, A> for JWTAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationDataWithUser, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }
        authorization::check_tenant(
            payload.tenant_id.clone(),
            &state.session_state().tenant.tenant_id,
        )?;
        let role_info = authorization::get_role_info(state, &payload).await?;
        authorization::check_permission(self.permission, &role_info)?;

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                &payload.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&payload.merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant account for the merchant id")?;

        let user_id = payload.user_id;

        let user = state
            .session_state()
            .global_store
            .find_user_by_id(&user_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch user for the user id")?;

        let auth = AuthenticationDataWithUser {
            merchant_account: merchant,
            key_store,
            profile_id: payload.profile_id.clone(),
            user,
        };

        let auth_type = AuthenticationType::MerchantJwt {
            merchant_id: auth.merchant_account.get_id().clone(),
            user_id: Some(user_id),
        };

        Ok((auth, auth_type))
    }
}

/// Validates whether the merchant account type is authorized to access the resource
///
/// # Access Control Logic:
/// - **Connected Merchant**: Allowed if `allow_connected_scope_operation` is true (no header required)
/// - **Platform Merchant**:
///   - With `X-Connected-Merchant-Id` header: Allowed if `allow_connected_scope_operation` is true
///     (platform acting on behalf of connected merchant)
///   - Without header: Allowed if `allow_platform_self_operation` is true
///     (platform self operation)
/// - **Standard Merchant**: Always allowed
pub fn check_merchant_access<A>(
    state: &A,
    request_headers: &HeaderMap,
    initiator_merchant_account_type: MerchantAccountType,
    allow_connected_scope_operation: bool,
    allow_platform_self_operation: bool,
) -> Result<(), error_stack::Report<errors::ApiErrorResponse>>
where
    A: SessionStateInfo + Sync,
{
    // Check if connected merchant header is present
    let has_connected_merchant_header = HeaderMapStruct::new(request_headers)
        .get_id_type_from_header_if_present::<id_type::MerchantId>(headers::X_CONNECTED_MERCHANT_ID)
        .map_err(|e| {
            e.change_context(errors::ApiErrorResponse::InvalidRequestData {
                message: "Invalid X-Connected-Merchant-Id header".to_string(),
            })
        })?
        .is_some();

    match initiator_merchant_account_type {
        MerchantAccountType::Connected => {
            // Check if platform feature is enabled
            state.conf().platform.enabled.then_some(()).ok_or_else(|| {
                report!(errors::ApiErrorResponse::ConnectedAccountAuthNotSupported)
                    .attach_printable("Platform feature is not enabled")
            })?;

            // Connected merchant can perform operation if allow_connected_scope_operation is true
            allow_connected_scope_operation
                .then_some(())
                .ok_or_else(|| {
                    report!(errors::ApiErrorResponse::ConnectedAccountAuthNotSupported)
                        .attach_printable(
                            "Connected Merchant is not authorized to access the resource",
                        )
                })
        }
        MerchantAccountType::Platform => {
            // Check if platform feature is enabled
            state.conf().platform.enabled.then_some(()).ok_or_else(|| {
                report!(errors::ApiErrorResponse::PlatformAccountAuthNotSupported)
                    .attach_printable("Platform feature is not enabled")
            })?;

            if has_connected_merchant_header {
                // Platform is acting on behalf of a connected merchant
                // Requires allow_connected_scope_operation to be true
                allow_connected_scope_operation.then_some(()).ok_or_else(|| {
                    report!(errors::ApiErrorResponse::ConnectedAccountAuthNotSupported)
                        .attach_printable("Platform is not authorized to perform this operation on behalf of connected merchant")
                })
            } else {
                // Platform is performing a self operation (no connected merchant header)
                // Requires allow_platform_self_operation to be true
                allow_platform_self_operation.then_some(()).ok_or_else(|| {
                    report!(errors::ApiErrorResponse::PlatformAccountAuthNotSupported)
                        .attach_printable(
                            "Platform Merchant is not authorized to access the resource",
                        )
                })
            }
        }
        MerchantAccountType::Standard => Ok(()),
    }
}

/// Validates whether the merchant account type is authorized to access the resource for JWT authentication
///
/// # Access Control Logic for JWT:
/// - **Connected Merchant**: Allowed if `allow_connected` is true
/// - **Platform Merchant**: Allowed if `allow_platform` is true
/// - **Standard Merchant**: Always allowed
pub fn check_merchant_access_for_jwt<A>(
    state: &A,
    initiator_merchant_account_type: MerchantAccountType,
    allow_connected: bool,
    allow_platform: bool,
) -> Result<(), error_stack::Report<errors::ApiErrorResponse>>
where
    A: SessionStateInfo + Sync,
{
    match initiator_merchant_account_type {
        MerchantAccountType::Connected => {
            // Check if platform feature is enabled
            state.conf().platform.enabled.then_some(()).ok_or_else(|| {
                report!(errors::ApiErrorResponse::ConnectedAccountAuthNotSupported)
                    .attach_printable("Platform feature is not enabled")
            })?;

            // Connected merchant can perform operation if allow_connected is true
            allow_connected.then_some(()).ok_or_else(|| {
                report!(errors::ApiErrorResponse::ConnectedAccountAuthNotSupported)
                    .attach_printable("Connected Merchant is not authorized to access the resource")
            })
        }
        MerchantAccountType::Platform => {
            // Check if platform feature is enabled
            state.conf().platform.enabled.then_some(()).ok_or_else(|| {
                report!(errors::ApiErrorResponse::PlatformAccountAuthNotSupported)
                    .attach_printable("Platform feature is not enabled")
            })?;

            // Platform merchant can perform operation if allow_platform is true
            allow_platform.then_some(()).ok_or_else(|| {
                report!(errors::ApiErrorResponse::PlatformAccountAuthNotSupported)
                    .attach_printable("Platform Merchant is not authorized to access the resource")
            })
        }
        MerchantAccountType::Standard => Ok(()),
    }
}

/// Validates that a merchant account is a valid connected merchant for platform operations
fn validate_connected_merchant_account(
    connected_merchant_account: &domain::MerchantAccount,
    platform_org_id: id_type::OrganizationId,
) -> RouterResult<()> {
    (connected_merchant_account.organization_id == platform_org_id
        && connected_merchant_account.merchant_account_type == MerchantAccountType::Connected)
        .then_some(())
        .ok_or_else(|| {
            report!(errors::ApiErrorResponse::InvalidPlatformOperation)
                .attach_printable("Invalid connected merchant for platform operation")
        })
}

/// Fetches and validates the connected merchant account and key store for platform operations
async fn get_connected_account_and_key_store<A>(
    state: &A,
    connected_merchant_id: id_type::MerchantId,
    platform_org_id: id_type::OrganizationId,
) -> RouterResult<(domain::MerchantAccount, domain::MerchantKeyStore)>
where
    A: SessionStateInfo + Sync,
{
    let key_store = state
        .store()
        .get_merchant_key_store_by_merchant_id(
            &connected_merchant_id,
            &state.store().get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::InvalidPlatformOperation)
        .attach_printable("Failed to fetch merchant key store for the merchant id")?;

    let connected_merchant_account = state
        .store()
        .find_merchant_account_by_merchant_id(&connected_merchant_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::InvalidPlatformOperation)
        .attach_printable("Failed to fetch merchant account for the merchant id")?;

    validate_connected_merchant_account(&connected_merchant_account, platform_org_id)?;

    Ok((connected_merchant_account, key_store))
}

/// Resolves processor and provider merchant accounts based on merchant type and headers
///
/// This function handles the resolution of merchant accounts for platform operations:
/// - **Platform Merchant**: Uses `X-Connected-Merchant-Id` header to act on behalf of connected merchant,
///   or operates as self if header is absent
/// - **Connected Merchant**: Resolves the platform account for the connected merchant
/// - **Standard Merchant**: Returns the merchant as-is
///
/// Note: Access control validation should be done via `check_merchant_access` before calling this function
async fn resolve_platform<A>(
    state: &A,
    request_headers: &HeaderMap,
    initiator_merchant_account: domain::MerchantAccount,
    initiator_merchant_key_store: domain::MerchantKeyStore,
    initiator: Option<domain::Initiator>,
) -> RouterResult<domain::Platform>
where
    A: SessionStateInfo + Sync,
{
    let header_map = HeaderMapStruct::new(request_headers);

    let (processor_merchant_account, processor_key_store, platform_account_with_key_store) =
        match initiator_merchant_account.merchant_account_type {
            MerchantAccountType::Platform => {
                let connected_merchant_id = header_map
                    .get_id_type_from_header_if_present::<id_type::MerchantId>(
                        headers::X_CONNECTED_MERCHANT_ID,
                    )?;

                // If header present: platform acts on behalf of connected merchant
                // If header absent: platform operates on the platform-connected merchant group
                let (processor_merchant_account, processor_key_store) = match connected_merchant_id
                {
                    Some(connected_merchant_id) => {
                        get_connected_account_and_key_store(
                            state,
                            connected_merchant_id,
                            initiator_merchant_account.organization_id.clone(),
                        )
                        .await?
                    }
                    None => (
                        initiator_merchant_account.clone(),
                        initiator_merchant_key_store.clone(),
                    ),
                };

                (
                    processor_merchant_account,
                    processor_key_store,
                    Some(PlatformAccountWithKeyStore {
                        account: initiator_merchant_account.clone(),
                        key_store: initiator_merchant_key_store,
                    }),
                )
            }
            MerchantAccountType::Connected => {
                fp_utils::when(
                    header_map
                        .get_id_type_from_header_if_present::<id_type::MerchantId>(
                            headers::X_CONNECTED_MERCHANT_ID,
                        )?
                        .is_some(),
                    || {
                        Err(report!(errors::ApiErrorResponse::InvalidConnectedOperation))
                            .attach_printable(
                                "Connected merchant cannot use X-Connected-Merchant-Id header",
                            )
                    },
                )?;

                let (platform_account, platform_key_store) =
                    get_platform_account_and_key_store(state, &initiator_merchant_account).await?;

                (
                    initiator_merchant_account.clone(),
                    initiator_merchant_key_store,
                    Some(PlatformAccountWithKeyStore {
                        account: platform_account,
                        key_store: platform_key_store,
                    }),
                )
            }
            MerchantAccountType::Standard => {
                fp_utils::when(
                    header_map
                        .get_id_type_from_header_if_present::<id_type::MerchantId>(
                            headers::X_CONNECTED_MERCHANT_ID,
                        )?
                        .is_some(),
                    || {
                        Err(report!(errors::ApiErrorResponse::InvalidPlatformOperation))
                            .attach_printable(
                                "Standard merchant cannot use X-Connected-Merchant-Id header",
                            )
                    },
                )?;

                (
                    initiator_merchant_account.clone(),
                    initiator_merchant_key_store,
                    None,
                )
            }
        };

    let platform = match platform_account_with_key_store {
        Some(platform_account) => domain::Platform::new(
            platform_account.account,
            platform_account.key_store,
            processor_merchant_account,
            processor_key_store,
            initiator,
        ),
        None => domain::Platform::new(
            processor_merchant_account.clone(),
            processor_key_store.clone(),
            processor_merchant_account,
            processor_key_store,
            initiator,
        ),
    };

    Ok(platform)
}

/// Fetches the platform merchant account and key store
async fn get_platform_account_and_key_store<A>(
    state: &A,
    merchant_account: &domain::MerchantAccount,
) -> RouterResult<(domain::MerchantAccount, domain::MerchantKeyStore)>
where
    A: SessionStateInfo + Sync,
{
    let organization = state
        .session_state()
        .accounts_store
        .find_organization_by_org_id(merchant_account.get_org_id())
        .await
        .change_context(errors::ApiErrorResponse::InvalidPlatformOperation)
        .attach_printable("Failed to fetch organization for connected merchant")?;

    let platform_merchant_id = organization.platform_merchant_id.ok_or_else(|| {
        report!(errors::ApiErrorResponse::InvalidPlatformOperation)
            .attach_printable("Connected merchant missing platform merchant id")
    })?;

    let platform_key_store = state
        .store()
        .get_merchant_key_store_by_merchant_id(
            &platform_merchant_id,
            &state.store().get_master_key().to_vec().into(),
        )
        .await
        .change_context(errors::ApiErrorResponse::InvalidPlatformOperation)
        .attach_printable("Failed to fetch key store for platform merchant")?;

    let platform_account = state
        .store()
        .find_merchant_account_by_merchant_id(&platform_merchant_id, &platform_key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::InvalidPlatformOperation)?;

    (platform_account.is_platform_account())
        .then_some((platform_account, platform_key_store))
        .ok_or_else(|| {
            report!(errors::ApiErrorResponse::InvalidPlatformOperation)
                .attach_printable("Mapped platform merchant is not a platform account")
        })
}

fn throw_error_if_platform_merchant_authentication_required(
    request_headers: &HeaderMap,
) -> RouterResult<()> {
    HeaderMapStruct::new(request_headers)
        .get_id_type_from_header_if_present::<id_type::MerchantId>(
            headers::X_CONNECTED_MERCHANT_ID,
        )?
        .map_or(Ok(()), |_| {
            Err(errors::ApiErrorResponse::PlatformAccountAuthNotSupported.into())
        })
}

#[cfg(feature = "recon")]
#[async_trait]
impl<A> AuthenticateAndFetch<UserFromTokenWithRoleInfo, A> for JWTAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(UserFromTokenWithRoleInfo, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if payload.check_in_blacklist(state).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }
        authorization::check_tenant(
            payload.tenant_id.clone(),
            &state.session_state().tenant.tenant_id,
        )?;
        let role_info = authorization::get_role_info(state, &payload).await?;
        authorization::check_permission(self.permission, &role_info)?;

        let user = UserFromToken {
            user_id: payload.user_id.clone(),
            merchant_id: payload.merchant_id.clone(),
            org_id: payload.org_id,
            role_id: payload.role_id,
            profile_id: payload.profile_id,
            tenant_id: payload.tenant_id,
        };

        Ok((
            UserFromTokenWithRoleInfo { user, role_info },
            AuthenticationType::MerchantJwt {
                merchant_id: payload.merchant_id,
                user_id: Some(payload.user_id),
            },
        ))
    }
}

#[cfg(feature = "recon")]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ReconToken {
    pub user_id: String,
    pub merchant_id: id_type::MerchantId,
    pub role_id: String,
    pub exp: u64,
    pub org_id: id_type::OrganizationId,
    pub profile_id: id_type::ProfileId,
    pub tenant_id: Option<id_type::TenantId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acl: Option<String>,
}

#[cfg(all(feature = "olap", feature = "recon"))]
impl ReconToken {
    pub async fn new_token(
        user_id: String,
        merchant_id: id_type::MerchantId,
        settings: &Settings,
        org_id: id_type::OrganizationId,
        profile_id: id_type::ProfileId,
        tenant_id: Option<id_type::TenantId>,
        role_info: authorization::roles::RoleInfo,
    ) -> UserResult<String> {
        let exp_duration = std::time::Duration::from_secs(consts::JWT_TOKEN_TIME_IN_SECS);
        let exp = jwt::generate_exp(exp_duration)?.as_secs();
        let acl = role_info.get_recon_acl();
        let optional_acl_str = serde_json::to_string(&acl)
            .inspect_err(|err| logger::error!("Failed to serialize acl to string: {}", err))
            .change_context(errors::UserErrors::InternalServerError)
            .attach_printable("Failed to serialize acl to string. Using empty ACL")
            .ok();
        let token_payload = Self {
            user_id,
            merchant_id,
            role_id: role_info.get_role_id().to_string(),
            exp,
            org_id,
            profile_id,
            tenant_id,
            acl: optional_acl_str,
        };
        jwt::generate_jwt(&token_payload, settings).await
    }
}
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ExternalToken {
    pub user_id: String,
    pub merchant_id: id_type::MerchantId,
    pub exp: u64,
    pub external_service_type: ExternalServiceType,
}

impl ExternalToken {
    pub async fn new_token(
        user_id: String,
        merchant_id: id_type::MerchantId,
        settings: &Settings,
        external_service_type: ExternalServiceType,
    ) -> UserResult<String> {
        let exp_duration = std::time::Duration::from_secs(consts::JWT_TOKEN_TIME_IN_SECS);
        let exp = jwt::generate_exp(exp_duration)?.as_secs();

        let token_payload = Self {
            user_id,
            merchant_id,
            exp,
            external_service_type,
        };
        jwt::generate_jwt(&token_payload, settings).await
    }

    pub fn check_service_type(
        &self,
        required_service_type: &ExternalServiceType,
    ) -> RouterResult<()> {
        Ok(fp_utils::when(
            &self.external_service_type != required_service_type,
            || {
                Err(errors::ApiErrorResponse::AccessForbidden {
                    resource: required_service_type.to_string(),
                })
            },
        )?)
    }
}
