use actix_web::http::header::HeaderMap;
#[cfg(feature = "payouts")]
use api_models::payouts;
use api_models::{
    payment_methods::{PaymentMethodCreate, PaymentMethodListRequest},
    payments,
};
use async_trait::async_trait;
use common_enums::TokenPurpose;
use common_utils::{date_time, id_type};
use error_stack::{report, ResultExt};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use masking::PeekInterface;
use router_env::logger;
use serde::Serialize;

use self::blacklist::BlackList;
#[cfg(feature = "partial-auth")]
use self::detached::{ExtractedPayload, GetAuthType};
use super::authorization::{self, permissions::Permission};
#[cfg(feature = "olap")]
use super::jwt;
#[cfg(feature = "recon")]
use super::recon::ReconToken;
#[cfg(feature = "olap")]
use crate::configs::Settings;
#[cfg(feature = "olap")]
use crate::consts;
#[cfg(feature = "olap")]
use crate::core::errors::UserResult;
#[cfg(feature = "partial-auth")]
use crate::core::metrics;
#[cfg(feature = "recon")]
use crate::routes::SessionState;
use crate::{
    core::{
        api_keys,
        errors::{self, utils::StorageErrorExt, RouterResult},
    },
    headers,
    routes::app::SessionStateInfo,
    services::api,
    types::domain,
    utils::OptionExt,
};

pub mod blacklist;
pub mod cookies;
pub mod decision;

#[cfg(feature = "partial-auth")]
mod detached;

#[derive(Clone, Debug)]
pub struct AuthenticationData {
    pub merchant_account: domain::MerchantAccount,
    pub key_store: domain::MerchantKeyStore,
    pub profile_id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct AuthenticationDataWithMultipleProfiles {
    pub merchant_account: domain::MerchantAccount,
    pub key_store: domain::MerchantKeyStore,
    pub profile_id_list: Option<Vec<String>>,
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
        key_id: String,
    },
    AdminApiKey,
    AdminApiAuthWithMerchantId {
        merchant_id: id_type::MerchantId,
    },
    MerchantJwt {
        merchant_id: id_type::MerchantId,
        user_id: Option<String>,
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
    WebhookAuth {
        merchant_id: id_type::MerchantId,
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
            | Self::WebhookAuth { merchant_id } => Some(merchant_id),
            Self::AdminApiKey
            | Self::UserJwt { .. }
            | Self::SinglePurposeJwt { .. }
            | Self::SinglePurposeOrLoginJwt { .. }
            | Self::NoAuth => None,
        }
    }
}

#[cfg(feature = "olap")]
#[derive(Clone, Debug)]
pub struct UserFromSinglePurposeToken {
    pub user_id: String,
    pub origin: domain::Origin,
    pub path: Vec<TokenPurpose>,
}

#[cfg(feature = "olap")]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SinglePurposeToken {
    pub user_id: String,
    pub purpose: TokenPurpose,
    pub origin: domain::Origin,
    pub path: Vec<TokenPurpose>,
    pub exp: u64,
}

#[cfg(feature = "olap")]
impl SinglePurposeToken {
    pub async fn new_token(
        user_id: String,
        purpose: TokenPurpose,
        origin: domain::Origin,
        settings: &Settings,
        path: Vec<TokenPurpose>,
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
    pub profile_id: Option<String>,
}

#[cfg(feature = "olap")]
impl AuthToken {
    pub async fn new_token(
        user_id: String,
        merchant_id: id_type::MerchantId,
        role_id: String,
        settings: &Settings,
        org_id: id_type::OrganizationId,
        profile_id: Option<String>,
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
    pub profile_id: Option<String>,
}

pub struct UserIdFromAuth {
    pub user_id: String,
}

#[cfg(feature = "olap")]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SinglePurposeOrLoginToken {
    pub user_id: String,
    pub role_id: Option<String>,
    pub purpose: Option<TokenPurpose>,
    pub exp: u64,
}

pub trait AuthInfo {
    fn get_merchant_id(&self) -> Option<&id_type::MerchantId>;
}

impl AuthInfo for () {
    fn get_merchant_id(&self) -> Option<&id_type::MerchantId> {
        None
    }
}

impl AuthInfo for AuthenticationData {
    fn get_merchant_id(&self) -> Option<&id_type::MerchantId> {
        Some(self.merchant_account.get_id())
    }
}

impl AuthInfo for AuthenticationDataWithMultipleProfiles {
    fn get_merchant_id(&self) -> Option<&id_type::MerchantId> {
        Some(self.merchant_account.get_id())
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

#[derive(Debug)]
pub struct ApiKeyAuth;

pub struct NoAuth;

#[cfg(feature = "partial-auth")]
impl GetAuthType for ApiKeyAuth {
    fn get_auth_type(&self) -> detached::PayloadType {
        detached::PayloadType::ApiKey
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

        let key_manager_state = &(&state.session_state()).into();

        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                key_manager_state,
                &stored_api_key.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .change_context(errors::ApiErrorResponse::Unauthorized)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(
                key_manager_state,
                &stored_api_key.merchant_id,
                &key_store,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let auth = AuthenticationData {
            merchant_account: merchant,
            key_store,
            profile_id: None,
        };
        Ok((
            auth.clone(),
            AuthenticationType::ApiKey {
                merchant_id: auth.merchant_account.get_id().clone(),
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

#[cfg(feature = "partial-auth")]
#[async_trait]
impl<A, I> AuthenticateAndFetch<AuthenticationData, A> for HeaderAuth<I>
where
    A: SessionStateInfo + Sync,
    I: AuthenticateAndFetch<AuthenticationData, A> + GetAuthType + Sync + Send,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let report_failure = || {
            metrics::PARTIAL_AUTH_FAILURE.add(&metrics::CONTEXT, 1, &[]);
        };

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
                    let auth = construct_authentication_data(state, &merchant_id).await?;
                    Ok((
                        auth.clone(),
                        AuthenticationType::ApiKey {
                            merchant_id: auth.merchant_account.get_id().clone(),
                            key_id,
                        },
                    ))
                }
                ExtractedPayload {
                    payload_type: detached::PayloadType::PublishableKey,
                    merchant_id: Some(merchant_id),
                    key_id: None,
                } => {
                    let auth = construct_authentication_data(state, &merchant_id).await?;
                    Ok((
                        auth.clone(),
                        AuthenticationType::PublishableKey {
                            merchant_id: auth.merchant_account.get_id().clone(),
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

#[cfg(feature = "partial-auth")]
async fn construct_authentication_data<A>(
    state: &A,
    merchant_id: &id_type::MerchantId,
) -> RouterResult<AuthenticationData>
where
    A: SessionStateInfo,
{
    let key_store = state
        .store()
        .get_merchant_key_store_by_merchant_id(
            &(&state.session_state()).into(),
            merchant_id,
            &state.store().get_master_key().to_vec().into(),
        )
        .await
        .change_context(errors::ApiErrorResponse::Unauthorized)
        .attach_printable("Failed to fetch merchant key store for the merchant id")?;

    let merchant = state
        .store()
        .find_merchant_account_by_merchant_id(
            &(&state.session_state()).into(),
            merchant_id,
            &key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

    let auth = AuthenticationData {
        merchant_account: merchant,
        key_store,
        profile_id: None,
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

        if self.0 != payload.purpose {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }

        Ok((
            UserFromSinglePurposeToken {
                user_id: payload.user_id.clone(),
                origin: payload.origin.clone(),
                path: payload.path,
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

        if self.0 != payload.purpose {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }

        Ok((
            Some(UserFromSinglePurposeToken {
                user_id: payload.user_id.clone(),
                origin: payload.origin.clone(),
                path: payload.path,
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
pub struct AdminApiAuthWithMerchantId(AdminApiAuth);

#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for AdminApiAuthWithMerchantId
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        self.0
            .authenticate_and_fetch(request_headers, state)
            .await?;
        let merchant_id =
            get_header_value_by_key(headers::X_MERCHANT_ID.to_string(), request_headers)?
                .get_required_value(headers::X_MERCHANT_ID)
                .change_context(errors::ApiErrorResponse::InvalidRequestData {
                    message: format!("`{}` header is missing", headers::X_MERCHANT_ID),
                })
                .and_then(|merchant_id_str| {
                    id_type::MerchantId::try_from(std::borrow::Cow::from(
                        merchant_id_str.to_string(),
                    ))
                    .change_context(
                        errors::ApiErrorResponse::InvalidRequestData {
                            message: format!("`{}` header is invalid", headers::X_MERCHANT_ID),
                        },
                    )
                })?;
        let key_manager_state = &(&state.session_state()).into();
        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                key_manager_state,
                &merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .map_err(|e| {
                if e.current_context().is_db_not_found() {
                    e.change_context(errors::ApiErrorResponse::Unauthorized)
                } else {
                    e.change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to fetch merchant key store for the merchant id")
                }
            })?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(key_manager_state, &merchant_id, &key_store)
            .await
            .map_err(|e| {
                if e.current_context().is_db_not_found() {
                    e.change_context(errors::ApiErrorResponse::Unauthorized)
                } else {
                    e.change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to fetch merchant account for the merchant id")
                }
            })?;

        let auth = AuthenticationData {
            merchant_account: merchant,
            key_store,
            profile_id: None,
        };
        Ok((
            auth,
            AuthenticationType::AdminApiAuthWithMerchantId { merchant_id },
        ))
    }
}

#[derive(Debug)]
pub struct EphemeralKeyAuth;

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
pub struct MerchantIdAuth(pub id_type::MerchantId);

#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for MerchantIdAuth
where
    A: SessionStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        _request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let key_manager_state = &(&state.session_state()).into();
        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                key_manager_state,
                &self.0,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .map_err(|e| {
                if e.current_context().is_db_not_found() {
                    e.change_context(errors::ApiErrorResponse::Unauthorized)
                } else {
                    e.change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to fetch merchant key store for the merchant id")
                }
            })?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(key_manager_state, &self.0, &key_store)
            .await
            .map_err(|e| {
                if e.current_context().is_db_not_found() {
                    e.change_context(errors::ApiErrorResponse::Unauthorized)
                } else {
                    e.change_context(errors::ApiErrorResponse::InternalServerError)
                }
            })?;

        let auth = AuthenticationData {
            merchant_account: merchant,
            key_store,
            profile_id: None,
        };
        Ok((
            auth.clone(),
            AuthenticationType::MerchantId {
                merchant_id: auth.merchant_account.get_id().clone(),
            },
        ))
    }
}

#[derive(Debug)]
pub struct PublishableKeyAuth;

#[cfg(feature = "partial-auth")]
impl GetAuthType for PublishableKeyAuth {
    fn get_auth_type(&self) -> detached::PayloadType {
        detached::PayloadType::PublishableKey
    }
}

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
        let key_manager_state = &(&state.session_state()).into();
        state
            .store()
            .find_merchant_account_by_publishable_key(key_manager_state, publishable_key)
            .await
            .map_err(|e| {
                if e.current_context().is_db_not_found() {
                    e.change_context(errors::ApiErrorResponse::Unauthorized)
                } else {
                    e.change_context(errors::ApiErrorResponse::InternalServerError)
                }
            })
            .map(|auth| {
                (
                    auth.clone(),
                    AuthenticationType::PublishableKey {
                        merchant_id: auth.merchant_account.get_id().clone(),
                    },
                )
            })
    }
}

#[derive(Debug)]
pub(crate) struct JWTAuth(pub Permission);

#[derive(serde::Deserialize)]
struct JwtAuthPayloadFetchUnit {
    #[serde(rename(deserialize = "exp"))]
    _exp: u64,
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

        let permissions = authorization::get_permissions(state, &payload).await?;
        authorization::check_authorization(&self.0, &permissions)?;

        Ok((
            (),
            AuthenticationType::MerchantJwt {
                merchant_id: payload.merchant_id,
                user_id: Some(payload.user_id),
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

        let permissions = authorization::get_permissions(state, &payload).await?;
        authorization::check_authorization(&self.0, &permissions)?;

        Ok((
            UserFromToken {
                user_id: payload.user_id.clone(),
                merchant_id: payload.merchant_id.clone(),
                org_id: payload.org_id,
                role_id: payload.role_id,
                profile_id: payload.profile_id,
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

        let permissions = authorization::get_permissions(state, &payload).await?;
        authorization::check_authorization(&self.0, &permissions)?;
        let key_manager_state = &(&state.session_state()).into();
        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                key_manager_state,
                &payload.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .change_context(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(
                key_manager_state,
                &payload.merchant_id,
                &key_store,
            )
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

pub struct JWTAuthMerchantFromRoute {
    pub merchant_id: id_type::MerchantId,
    pub required_permission: Permission,
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

        let permissions = authorization::get_permissions(state, &payload).await?;
        authorization::check_authorization(&self.required_permission, &permissions)?;

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

        if payload.merchant_id != self.merchant_id {
            return Err(report!(errors::ApiErrorResponse::InvalidJwtToken));
        }

        let permissions = authorization::get_permissions(state, &payload).await?;
        authorization::check_authorization(&self.required_permission, &permissions)?;
        let key_manager_state = &(&state.session_state()).into();
        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                key_manager_state,
                &payload.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(
                key_manager_state,
                &payload.merchant_id,
                &key_store,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant account for the merchant id")?;

        let auth = AuthenticationData {
            merchant_account: merchant,
            key_store,
            profile_id: payload.profile_id,
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

pub async fn parse_jwt_payload<A, T>(headers: &HeaderMap, state: &A) -> RouterResult<T>
where
    T: serde::de::DeserializeOwned,
    A: SessionStateInfo + Sync,
{
    let token = match get_cookie_from_header(headers).and_then(cookies::parse_cookie) {
        Ok(cookies) => cookies,
        Err(error) => {
            let token = get_jwt_from_authorization_header(headers);
            if token.is_err() {
                logger::error!(?error);
            }
            token?.to_owned()
        }
    };
    decode_jwt(&token, state).await
}

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

        let permissions = authorization::get_permissions(state, &payload).await?;
        authorization::check_authorization(&self.0, &permissions)?;
        let key_manager_state = &(&state.session_state()).into();
        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                key_manager_state,
                &payload.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(
                key_manager_state,
                &payload.merchant_id,
                &key_store,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant account for the merchant id")?;

        let auth = AuthenticationData {
            merchant_account: merchant,
            key_store,
            profile_id: payload.profile_id,
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

pub type AuthenticationDataWithUserId = (AuthenticationData, String);

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

        let permissions = authorization::get_permissions(state, &payload).await?;
        authorization::check_authorization(&self.0, &permissions)?;
        let key_manager_state = &(&state.session_state()).into();
        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                key_manager_state,
                &payload.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .change_context(errors::ApiErrorResponse::InvalidJwtToken)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(
                key_manager_state,
                &payload.merchant_id,
                &key_store,
            )
            .await
            .change_context(errors::ApiErrorResponse::InvalidJwtToken)?;

        let auth = AuthenticationData {
            merchant_account: merchant,
            key_store,
            profile_id: payload.profile_id,
        };
        Ok((
            (auth.clone(), payload.user_id.clone()),
            AuthenticationType::MerchantJwt {
                merchant_id: auth.merchant_account.get_id().clone(),
                user_id: None,
            },
        ))
    }
}

pub struct DashboardNoPermissionAuth;

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

        Ok((
            UserFromToken {
                user_id: payload.user_id.clone(),
                merchant_id: payload.merchant_id.clone(),
                org_id: payload.org_id,
                role_id: payload.role_id,
                profile_id: payload.profile_id,
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

        Ok(((), AuthenticationType::NoAuth))
    }
}

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
        let key_manager_state = &(&state.session_state()).into();
        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                key_manager_state,
                &payload.merchant_id,
                &state.store().get_master_key().to_vec().into(),
            )
            .await
            .change_context(errors::ApiErrorResponse::Unauthorized)
            .attach_printable("Failed to fetch merchant key store for the merchant id")?;

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(
                key_manager_state,
                &payload.merchant_id,
                &key_store,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let auth = AuthenticationData {
            merchant_account: merchant,
            key_store,
            profile_id: payload.profile_id,
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

pub trait ClientSecretFetch {
    fn get_client_secret(&self) -> Option<&String>;
}
#[cfg(feature = "payouts")]
impl ClientSecretFetch for payouts::PayoutCreateRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref()
    }
}

impl ClientSecretFetch for payments::PaymentsRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref()
    }
}

impl ClientSecretFetch for PaymentMethodListRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref()
    }
}

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

impl ClientSecretFetch for payments::PaymentsRetrieveRequest {
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

impl ClientSecretFetch for api_models::payment_methods::PaymentMethodUpdate {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref()
    }
}

pub fn get_auth_type_and_flow<A: SessionStateInfo + Sync + Send>(
    headers: &HeaderMap,
) -> RouterResult<(
    Box<dyn AuthenticateAndFetch<AuthenticationData, A>>,
    api::AuthFlow,
)> {
    let api_key = get_api_key(headers)?;

    if api_key.starts_with("pk_") {
        return Ok((
            Box::new(HeaderAuth(PublishableKeyAuth)),
            api::AuthFlow::Client,
        ));
    }
    Ok((Box::new(HeaderAuth(ApiKeyAuth)), api::AuthFlow::Merchant))
}

pub fn check_client_secret_and_get_auth<T>(
    headers: &HeaderMap,
    payload: &impl ClientSecretFetch,
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
            Box::new(HeaderAuth(PublishableKeyAuth)),
            api::AuthFlow::Client,
        ));
    }

    if payload.get_client_secret().is_some() {
        return Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "client_secret is not a valid parameter".to_owned(),
        }
        .into());
    }
    Ok((Box::new(HeaderAuth(ApiKeyAuth)), api::AuthFlow::Merchant))
}

pub async fn get_ephemeral_or_other_auth<T>(
    headers: &HeaderMap,
    is_merchant_flow: bool,
    payload: Option<&impl ClientSecretFetch>,
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
            Box::new(HeaderAuth(ApiKeyAuth)),
            api::AuthFlow::Merchant,
            false,
        ))
    } else {
        let payload = payload.get_required_value("ClientSecretFetch")?;
        let (auth, auth_flow) = check_client_secret_and_get_auth(headers, payload)?;
        Ok((auth, auth_flow, false))
    }
}

pub fn is_ephemeral_auth<A: SessionStateInfo + Sync + Send>(
    headers: &HeaderMap,
) -> RouterResult<Box<dyn AuthenticateAndFetch<AuthenticationData, A>>> {
    let api_key = get_api_key(headers)?;

    if !api_key.starts_with("epk") {
        Ok(Box::new(HeaderAuth(ApiKeyAuth)))
    } else {
        Ok(Box::new(EphemeralKeyAuth))
    }
}

pub fn is_jwt_auth(headers: &HeaderMap) -> bool {
    headers.get(headers::AUTHORIZATION).is_some()
        || get_cookie_from_header(headers)
            .and_then(cookies::parse_cookie)
            .is_ok()
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
        .change_context(errors::ApiErrorResponse::InvalidJwtToken)
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
                    "Failed to convert header value to string for header key: {}",
                    key
                ))
        })
        .transpose()
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
    headers
        .get(cookies::get_cookie_header())
        .and_then(|header_value| header_value.to_str().ok())
        .ok_or(errors::ApiErrorResponse::InvalidCookie.into())
}

pub fn strip_jwt_token(token: &str) -> RouterResult<&str> {
    token
        .strip_prefix("Bearer ")
        .ok_or_else(|| errors::ApiErrorResponse::InvalidJwtToken.into())
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
pub struct ReconAdmin;

#[async_trait]
#[cfg(feature = "recon")]
impl<A> AuthenticateAndFetch<(), A> for ReconAdmin
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

        let admin_api_key = conf.secrets.get_inner().recon_admin_api_key.peek();

        if request_admin_api_key != admin_api_key {
            Err(report!(errors::ApiErrorResponse::Unauthorized)
                .attach_printable("Recon Admin Authentication Failure"))?;
        }

        Ok(((), AuthenticationType::NoAuth))
    }
}

#[cfg(feature = "recon")]
pub struct ReconJWT;
#[cfg(feature = "recon")]
pub struct ReconUser {
    pub user_id: String,
}
#[cfg(feature = "recon")]
impl AuthInfo for ReconUser {
    fn get_merchant_id(&self) -> Option<&id_type::MerchantId> {
        None
    }
}
#[cfg(all(feature = "olap", feature = "recon"))]
#[async_trait]
impl AuthenticateAndFetch<ReconUser, SessionState> for ReconJWT {
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &SessionState,
    ) -> RouterResult<(ReconUser, AuthenticationType)> {
        let payload = parse_jwt_payload::<SessionState, ReconToken>(request_headers, state).await?;

        Ok((
            ReconUser {
                user_id: payload.user_id,
            },
            AuthenticationType::NoAuth,
        ))
    }
}
