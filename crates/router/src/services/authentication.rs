use actix_web::http::header::HeaderMap;
use api_models::{payment_methods::PaymentMethodListRequest, payments::PaymentsRequest};
use async_trait::async_trait;
use common_utils::date_time;
use error_stack::{report, IntoReport, ResultExt};
#[cfg(feature = "kms")]
use external_services::kms;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use masking::{PeekInterface, StrongSecret};

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

pub struct AuthenticationData {
    pub merchant_account: domain::MerchantAccount,
    pub key_store: domain::MerchantKeyStore,
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
    T: AuthInfo,
{
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<T>;
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
    ) -> RouterResult<()> {
        Ok(())
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
    ) -> RouterResult<AuthenticationData> {
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
                &config.kms,
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

        Ok(AuthenticationData {
            merchant_account: merchant,
            key_store,
        })
    }
}

static ADMIN_API_KEY: tokio::sync::OnceCell<StrongSecret<String>> =
    tokio::sync::OnceCell::const_new();

pub async fn get_admin_api_key(
    secrets: &settings::Secrets,
    #[cfg(feature = "kms")] kms_config: &kms::KmsConfig,
) -> RouterResult<&'static StrongSecret<String>> {
    ADMIN_API_KEY
        .get_or_try_init(|| async {
            #[cfg(feature = "kms")]
            let admin_api_key = kms::get_kms_client(kms_config)
                .await
                .decrypt(&secrets.kms_encrypted_admin_api_key)
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
    ) -> RouterResult<()> {
        let request_admin_api_key =
            get_api_key(request_headers).change_context(errors::ApiErrorResponse::Unauthorized)?;
        let conf = state.conf();

        let admin_api_key = get_admin_api_key(
            &conf.secrets,
            #[cfg(feature = "kms")]
            &conf.kms,
        )
        .await?;

        if request_admin_api_key != admin_api_key.peek() {
            Err(report!(errors::ApiErrorResponse::Unauthorized)
                .attach_printable("Admin Authentication Failure"))?;
        }

        Ok(())
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
    ) -> RouterResult<AuthenticationData> {
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

        Ok(AuthenticationData {
            merchant_account: merchant,
            key_store,
        })
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
    ) -> RouterResult<AuthenticationData> {
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
    }
}

#[derive(Debug)]
pub(crate) struct JWTAuth;

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
    ) -> RouterResult<()> {
        let mut token = get_jwt(request_headers)?;
        token = strip_jwt_token(token)?;
        decode_jwt::<JwtAuthPayloadFetchUnit>(token, state)
            .await
            .map(|_| ())
    }
}

#[derive(serde::Deserialize)]
struct JwtAuthPayloadFetchMerchantAccount {
    merchant_id: String,
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
    ) -> RouterResult<AuthenticationData> {
        let mut token = get_jwt(request_headers)?;
        token = strip_jwt_token(token)?;
        let payload = decode_jwt::<JwtAuthPayloadFetchMerchantAccount>(token, state).await?;
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

        Ok(AuthenticationData {
            merchant_account: merchant,
            key_store,
        })
    }
}

pub trait ClientSecretFetch {
    fn get_client_secret(&self) -> Option<&String>;
}

impl ClientSecretFetch for PaymentsRequest {
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
    #[cfg(feature = "kms")] kms_config: &kms::KmsConfig,
) -> RouterResult<&'static StrongSecret<String>> {
    JWT_SECRET
        .get_or_try_init(|| async {
            #[cfg(feature = "kms")]
            let jwt_secret = kms::get_kms_client(kms_config)
                .await
                .decrypt(&secrets.kms_encrypted_jwt_secret)
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
        &conf.kms,
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
    headers
        .get("api-key")
        .get_required_value("api-key")?
        .to_str()
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to convert API key to string")
}

pub fn get_jwt(headers: &HeaderMap) -> RouterResult<&str> {
    headers
        .get(crate::headers::AUTHORIZATION)
        .get_required_value(crate::headers::AUTHORIZATION)?
        .to_str()
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to convert JWT token to string")
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
