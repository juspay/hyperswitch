use actix_web::http::header::HeaderMap;
use api_models::{payment_methods::PaymentMethodListRequest, payments};
use async_trait::async_trait;
use common_utils::date_time;
use error_stack::{report, IntoReport, ResultExt};
#[cfg(feature = "kms")]
use external_services::kms::{self, decrypt::KmsDecrypt};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use masking::{PeekInterface, StrongSecret};
use serde::Serialize;

use super::authorization::{self, permissions::Permission};
#[cfg(feature = "olap")]
use super::jwt;
#[cfg(feature = "olap")]
use crate::consts;
#[cfg(feature = "olap")]
use crate::core::errors::UserResult;
use crate::{
    configs::settings,
    core::{
        api_keys,
        errors::{self, utils::StorageErrorExt, RouterResult},
    },
    db::StorageInterface,
    routes::app::AppStateInfo,
    services::api,
    types::domain,
    utils::OptionExt,
};

#[derive(Clone, Debug)]
pub struct AuthenticationData {
    pub merchant_account: domain::MerchantAccount,
    pub key_store: domain::MerchantKeyStore,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(
    tag = "api_auth_type",
    content = "authentication_data",
    rename_all = "snake_case"
)]
pub enum AuthenticationType {
    ApiKey {
        merchant_id: String,
        key_id: String,
    },
    AdminApiKey,
    MerchantJwt {
        merchant_id: String,
        user_id: Option<String>,
    },
    MerchantId {
        merchant_id: String,
    },
    PublishableKey {
        merchant_id: String,
    },
    WebhookAuth {
        merchant_id: String,
    },
    NoAuth,
}

impl AuthenticationType {
    pub fn get_merchant_id(&self) -> Option<&str> {
        match self {
            Self::ApiKey {
                merchant_id,
                key_id: _,
            }
            | Self::MerchantId { merchant_id }
            | Self::PublishableKey { merchant_id }
            | Self::MerchantJwt {
                merchant_id,
                user_id: _,
            }
            | Self::WebhookAuth { merchant_id } => Some(merchant_id.as_ref()),
            Self::AdminApiKey | Self::NoAuth => None,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct AuthToken {
    pub user_id: String,
    pub merchant_id: String,
    pub role_id: String,
    pub exp: u64,
    pub org_id: String,
}

#[cfg(feature = "olap")]
impl AuthToken {
    pub async fn new_token(
        user_id: String,
        merchant_id: String,
        role_id: String,
        settings: &settings::Settings,
        org_id: String,
    ) -> UserResult<String> {
        let exp_duration = std::time::Duration::from_secs(consts::JWT_TOKEN_TIME_IN_SECS);
        let exp = jwt::generate_exp(exp_duration)?.as_secs();
        let token_payload = Self {
            user_id,
            merchant_id,
            role_id,
            exp,
            org_id,
        };
        jwt::generate_jwt(&token_payload, settings).await
    }
}

#[derive(Clone)]
pub struct UserFromToken {
    pub user_id: String,
    pub merchant_id: String,
    pub role_id: String,
    pub org_id: String,
}

pub trait AuthInfo {
    fn get_merchant_id(&self) -> Option<&str>;
}

impl AuthInfo for () {
    fn get_merchant_id(&self) -> Option<&str> {
        None
    }
}

impl AuthInfo for AuthenticationData {
    fn get_merchant_id(&self) -> Option<&str> {
        Some(&self.merchant_account.merchant_id)
    }
}

#[async_trait]
pub trait AuthenticateAndFetch<T, A>
where
    A: AppStateInfo,
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

#[async_trait]
impl<A> AuthenticateAndFetch<(), A> for NoAuth
where
    A: AppStateInfo + Sync,
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
impl<A> AuthenticateAndFetch<AuthenticationData, A> for ApiKeyAuth
where
    A: AppStateInfo + Sync,
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
                .into_report()
                .attach_printable("API key is empty");
        }

        let api_key = api_keys::PlaintextApiKey::from(api_key);
        let hash_key = {
            let config = state.conf();
            api_keys::get_hash_key(
                &config.api_keys,
                #[cfg(feature = "kms")]
                kms::get_kms_client(&config.kms).await,
            )
            .await?
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

        let merchant = state
            .store()
            .find_merchant_account_by_merchant_id(&stored_api_key.merchant_id, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::Unauthorized)?;

        let auth = AuthenticationData {
            merchant_account: merchant,
            key_store,
        };
        Ok((
            auth.clone(),
            AuthenticationType::ApiKey {
                merchant_id: auth.merchant_account.merchant_id.clone(),
                key_id: stored_api_key.key_id,
            },
        ))
    }
}

static ADMIN_API_KEY: tokio::sync::OnceCell<StrongSecret<String>> =
    tokio::sync::OnceCell::const_new();

pub async fn get_admin_api_key(
    secrets: &settings::Secrets,
    #[cfg(feature = "kms")] kms_client: &kms::KmsClient,
) -> RouterResult<&'static StrongSecret<String>> {
    ADMIN_API_KEY
        .get_or_try_init(|| async {
            #[cfg(feature = "kms")]
            let admin_api_key = secrets
                .kms_encrypted_admin_api_key
                .decrypt_inner(kms_client)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to KMS decrypt admin API key")?;

            #[cfg(not(feature = "kms"))]
            let admin_api_key = secrets.admin_api_key.clone();

            Ok(StrongSecret::new(admin_api_key))
        })
        .await
}

#[derive(Debug)]
pub struct AdminApiAuth;

#[async_trait]
impl<A> AuthenticateAndFetch<(), A> for AdminApiAuth
where
    A: AppStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<((), AuthenticationType)> {
        let request_admin_api_key =
            get_api_key(request_headers).change_context(errors::ApiErrorResponse::Unauthorized)?;
        let conf = state.conf();

        let admin_api_key = get_admin_api_key(
            &conf.secrets,
            #[cfg(feature = "kms")]
            kms::get_kms_client(&conf.kms).await,
        )
        .await?;

        if request_admin_api_key != admin_api_key.peek() {
            Err(report!(errors::ApiErrorResponse::Unauthorized)
                .attach_printable("Admin Authentication Failure"))?;
        }

        Ok(((), AuthenticationType::AdminApiKey))
    }
}

#[derive(Debug)]
pub struct MerchantIdAuth(pub String);

#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for MerchantIdAuth
where
    A: AppStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        _request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let key_store = state
            .store()
            .get_merchant_key_store_by_merchant_id(
                self.0.as_ref(),
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
            .find_merchant_account_by_merchant_id(self.0.as_ref(), &key_store)
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
        };
        Ok((
            auth.clone(),
            AuthenticationType::MerchantId {
                merchant_id: auth.merchant_account.merchant_id.clone(),
            },
        ))
    }
}

#[derive(Debug)]
pub struct PublishableKeyAuth;

#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for PublishableKeyAuth
where
    A: AppStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let publishable_key =
            get_api_key(request_headers).change_context(errors::ApiErrorResponse::Unauthorized)?;

        state
            .store()
            .find_merchant_account_by_publishable_key(publishable_key)
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
                        merchant_id: auth.merchant_account.merchant_id.clone(),
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
    A: AppStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<((), AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;

        let permissions = authorization::get_permissions(&payload.role_id)?;
        authorization::check_authorization(&self.0, permissions)?;

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
    A: AppStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(UserFromToken, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;

        let permissions = authorization::get_permissions(&payload.role_id)?;
        authorization::check_authorization(&self.0, permissions)?;

        Ok((
            UserFromToken {
                user_id: payload.user_id.clone(),
                merchant_id: payload.merchant_id.clone(),
                org_id: payload.org_id,
                role_id: payload.role_id,
            },
            AuthenticationType::MerchantJwt {
                merchant_id: payload.merchant_id,
                user_id: Some(payload.user_id),
            },
        ))
    }
}

pub struct JWTAuthMerchantFromRoute {
    pub merchant_id: String,
    pub required_permission: Permission,
}

#[async_trait]
impl<A> AuthenticateAndFetch<(), A> for JWTAuthMerchantFromRoute
where
    A: AppStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<((), AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;

        let permissions = authorization::get_permissions(&payload.role_id)?;
        authorization::check_authorization(&self.required_permission, permissions)?;

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

pub async fn parse_jwt_payload<A, T>(headers: &HeaderMap, state: &A) -> RouterResult<T>
where
    T: serde::de::DeserializeOwned,
    A: AppStateInfo + Sync,
{
    let token = get_jwt_from_authorization_header(headers)?;
    let payload = decode_jwt(token, state).await?;

    Ok(payload)
}

#[derive(serde::Deserialize)]
struct JwtAuthPayloadFetchMerchantAccount {
    merchant_id: String,
    role_id: String,
}

#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for JWTAuth
where
    A: AppStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let payload =
            parse_jwt_payload::<A, JwtAuthPayloadFetchMerchantAccount>(request_headers, state)
                .await?;

        let permissions = authorization::get_permissions(&payload.role_id)?;
        authorization::check_authorization(&self.0, permissions)?;

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

        let auth = AuthenticationData {
            merchant_account: merchant,
            key_store,
        };
        Ok((
            auth.clone(),
            AuthenticationType::MerchantJwt {
                merchant_id: auth.merchant_account.merchant_id.clone(),
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
    A: AppStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(UserFromToken, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;

        Ok((
            UserFromToken {
                user_id: payload.user_id.clone(),
                merchant_id: payload.merchant_id.clone(),
                org_id: payload.org_id,
                role_id: payload.role_id,
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
    A: AppStateInfo + Sync,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<((), AuthenticationType)> {
        parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;

        Ok(((), AuthenticationType::NoAuth))
    }
}

pub trait ClientSecretFetch {
    fn get_client_secret(&self) -> Option<&String>;
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

impl ClientSecretFetch for api_models::cards_info::CardsInfoRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref()
    }
}

impl ClientSecretFetch for api_models::payments::PaymentsRetrieveRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref()
    }
}

impl ClientSecretFetch for api_models::payments::RetrievePaymentLinkRequest {
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

pub fn get_auth_type_and_flow<A: AppStateInfo + Sync>(
    headers: &HeaderMap,
) -> RouterResult<(
    Box<dyn AuthenticateAndFetch<AuthenticationData, A>>,
    api::AuthFlow,
)> {
    let api_key = get_api_key(headers)?;

    if api_key.starts_with("pk_") {
        return Ok((Box::new(PublishableKeyAuth), api::AuthFlow::Client));
    }
    Ok((Box::new(ApiKeyAuth), api::AuthFlow::Merchant))
}

pub fn check_client_secret_and_get_auth<T>(
    headers: &HeaderMap,
    payload: &impl ClientSecretFetch,
) -> RouterResult<(
    Box<dyn AuthenticateAndFetch<AuthenticationData, T>>,
    api::AuthFlow,
)>
where
    T: AppStateInfo,
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
        return Ok((Box::new(PublishableKeyAuth), api::AuthFlow::Client));
    }

    if payload.get_client_secret().is_some() {
        return Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "client_secret is not a valid parameter".to_owned(),
        }
        .into());
    }
    Ok((Box::new(ApiKeyAuth), api::AuthFlow::Merchant))
}

pub async fn is_ephemeral_auth<A: AppStateInfo + Sync>(
    headers: &HeaderMap,
    db: &dyn StorageInterface,
    customer_id: &str,
) -> RouterResult<Box<dyn AuthenticateAndFetch<AuthenticationData, A>>> {
    let api_key = get_api_key(headers)?;

    if !api_key.starts_with("epk") {
        return Ok(Box::new(ApiKeyAuth));
    }

    let ephemeral_key = db
        .get_ephemeral_key(api_key)
        .await
        .change_context(errors::ApiErrorResponse::Unauthorized)?;

    if ephemeral_key.customer_id.ne(customer_id) {
        return Err(report!(errors::ApiErrorResponse::InvalidEphemeralKey));
    }

    Ok(Box::new(MerchantIdAuth(ephemeral_key.merchant_id)))
}

pub fn is_jwt_auth(headers: &HeaderMap) -> bool {
    headers.get(crate::headers::AUTHORIZATION).is_some()
}

static JWT_SECRET: tokio::sync::OnceCell<StrongSecret<String>> = tokio::sync::OnceCell::const_new();

pub async fn get_jwt_secret(
    secrets: &settings::Secrets,
    #[cfg(feature = "kms")] kms_client: &kms::KmsClient,
) -> RouterResult<&'static StrongSecret<String>> {
    JWT_SECRET
        .get_or_try_init(|| async {
            #[cfg(feature = "kms")]
            let jwt_secret = secrets
                .kms_encrypted_jwt_secret
                .decrypt_inner(kms_client)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to KMS decrypt JWT secret")?;

            #[cfg(not(feature = "kms"))]
            let jwt_secret = secrets.jwt_secret.clone();

            Ok(StrongSecret::new(jwt_secret))
        })
        .await
}

pub async fn decode_jwt<T>(token: &str, state: &impl AppStateInfo) -> RouterResult<T>
where
    T: serde::de::DeserializeOwned,
{
    let conf = state.conf();
    let secret = get_jwt_secret(
        &conf.secrets,
        #[cfg(feature = "kms")]
        kms::get_kms_client(&conf.kms).await,
    )
    .await?
    .peek()
    .as_bytes();

    let key = DecodingKey::from_secret(secret);
    decode::<T>(token, &key, &Validation::new(Algorithm::HS256))
        .map(|decoded| decoded.claims)
        .into_report()
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
                .into_report()
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
        .get(crate::headers::AUTHORIZATION)
        .get_required_value(crate::headers::AUTHORIZATION)?
        .to_str()
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to convert JWT token to string")?
        .strip_prefix("Bearer ")
        .ok_or(errors::ApiErrorResponse::InvalidJwtToken.into())
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
