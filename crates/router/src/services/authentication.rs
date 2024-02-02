use actix_web::http::header::HeaderMap;
use api_models::{payment_methods::PaymentMethodListRequest, payments};
use async_trait::async_trait;
use common_utils::date_time;
use error_stack::{report, IntoReport, ResultExt};
#[cfg(feature = "hashicorp-vault")]
use external_services::hashicorp_vault::decrypt::VaultFetch;
#[cfg(feature = "kms")]
use external_services::kms::{self, decrypt::KmsDecrypt};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
#[cfg(feature = "hashicorp-vault")]
use masking::ExposeInterface;
use masking::{PeekInterface, StrongSecret};
use serde::Serialize;

use super::authorization::{self, permissions::Permission};
#[cfg(feature = "olap")]
use super::jwt;
#[cfg(feature = "recon")]
use super::recon::ReconToken;
#[cfg(feature = "olap")]
use crate::consts;
#[cfg(feature = "olap")]
use crate::core::errors::UserResult;
#[cfg(feature = "recon")]
use crate::routes::AppState;
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
pub mod blacklist;

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
    UserJwt {
        user_id: String,
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
        /// This method returns the merchant ID associated with the enum variant, if applicable.
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
            Self::AdminApiKey | Self::UserJwt { .. } | Self::NoAuth => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct UserWithoutMerchantFromToken {
    pub user_id: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct UserAuthToken {
    pub user_id: String,
    pub exp: u64,
}

#[cfg(feature = "olap")]
impl UserAuthToken {
        /// Asynchronously generates a new JWT token for the specified user ID using the provided settings.
    pub async fn new_token(user_id: String, settings: &settings::Settings) -> UserResult<String> {
        let exp_duration = std::time::Duration::from_secs(consts::JWT_TOKEN_TIME_IN_SECS);
        let exp = jwt::generate_exp(exp_duration)?.as_secs();
        let token_payload = Self { user_id, exp };
        jwt::generate_jwt(&token_payload, settings).await
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
        /// Generates a new JWT token for the given user, merchant, role, and organization, using the provided settings.
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
        /// Retrieves the merchant ID associated with the current object, if available.
    fn get_merchant_id(&self) -> Option<&str> {
        None
    }
}

impl AuthInfo for AuthenticationData {
        /// This method returns the merchant ID associated with the current instance
    /// of the struct. If a merchant ID is present, it is returned as an Option
    /// containing a reference to a string. If no merchant ID is present, None
    /// is returned.
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
        /// Asynchronously authenticates the user and fetches the authentication type based on the provided request headers and state.
    ///
    /// # Arguments
    ///
    /// * `_request_headers` - A reference to the request headers.
    /// * `_state` - A reference to the state of type A.
    ///
    /// # Returns
    ///
    /// A `RouterResult` containing a tuple with the first element being an empty value and the second element being the authentication type, which is set to `AuthenticationType::NoAuth`.
    ///
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
        /// Asynchronously authenticates the request using the provided request headers and fetches the necessary authentication data and type. 
    /// 
    /// # Arguments
    /// 
    /// * `request_headers` - The headers of the incoming request.
    /// * `state` - The state containing the configuration and store needed for authentication.
    /// 
    /// # Returns
    /// 
    /// A `RouterResult` containing a tuple of `AuthenticationData` and `AuthenticationType` if authentication is successful, otherwise an error is returned.
    /// 
    /// # Errors
    /// 
    /// An error is returned if the API key is unauthorized, expired, or if there are failures in fetching the necessary authentication data.
    /// 
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
                #[cfg(feature = "hashicorp-vault")]
                external_services::hashicorp_vault::get_hashicorp_client(&config.hc_vault)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)?,
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

/// Asynchronously retrieves the admin API key either from the secrets configuration or through decryption and fetching from external services like KMS or HashiCorp Vault. If the "kms" feature is enabled, it decrypts the encrypted admin API key using the provided KMS client. If the "hashicorp-vault" feature is enabled, it fetches the admin API key from HashiCorp Vault after decryption. Returns a `RouterResult` containing a reference to the admin API key as a `StrongSecret` wrapped in an `Ok` variant, or an error if the retrieval process fails.
pub async fn get_admin_api_key(
    secrets: &settings::Secrets,
    #[cfg(feature = "kms")] kms_client: &kms::KmsClient,
    #[cfg(feature = "hashicorp-vault")]
    hc_client: &external_services::hashicorp_vault::HashiCorpVault,
) -> RouterResult<&'static StrongSecret<String>> {
    ADMIN_API_KEY
        .get_or_try_init(|| async {
            #[cfg(not(feature = "kms"))]
            let admin_api_key = secrets.admin_api_key.clone();

            #[cfg(feature = "kms")]
            let admin_api_key = secrets
                .kms_encrypted_admin_api_key
                .decrypt_inner(kms_client)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to KMS decrypt admin API key")?;

            #[cfg(feature = "hashicorp-vault")]
            let admin_api_key = masking::Secret::new(admin_api_key)
                .fetch_inner::<external_services::hashicorp_vault::Kv2>(hc_client)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to KMS decrypt admin API key")?
                .expose();

            Ok(StrongSecret::new(admin_api_key))
        })
        .await
}

#[derive(Debug)]
pub struct UserWithoutMerchantJWTAuth;

#[cfg(feature = "olap")]
#[async_trait]
impl<A> AuthenticateAndFetch<UserWithoutMerchantFromToken, A> for UserWithoutMerchantJWTAuth
where
    A: AppStateInfo + Sync,
{
        /// Authenticates the user based on the provided request headers and state, and fetches the user information from the token.
    ///
    /// # Arguments
    ///
    /// * `request_headers` - The request headers containing the authentication token.
    /// * `state` - The state object containing the necessary information for authentication.
    ///
    /// # Returns
    ///
    /// A tuple containing the user information fetched from the token and the authentication type.
    ///
    /// # Errors
    ///
    /// Returns an error if the user is found in the blacklist or if the JWT token is invalid.
    ///
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(UserWithoutMerchantFromToken, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, UserAuthToken>(request_headers, state).await?;
        if blacklist::check_user_in_blacklist(state, &payload.user_id, payload.exp).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }

        Ok((
            UserWithoutMerchantFromToken {
                user_id: payload.user_id.clone(),
            },
            AuthenticationType::UserJwt {
                user_id: payload.user_id,
            },
        ))
    }
}

#[derive(Debug)]
pub struct AdminApiAuth;

#[async_trait]
impl<A> AuthenticateAndFetch<(), A> for AdminApiAuth
where
    A: AppStateInfo + Sync,
{
        /// Asynchronously authenticates the request using the provided request headers and fetches the necessary data from the state. 
    /// Returns a result containing a tuple with an empty value and the authentication type upon success, or an error if the authentication fails.
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
            #[cfg(feature = "hashicorp-vault")]
            external_services::hashicorp_vault::get_hashicorp_client(&conf.hc_vault)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed while getting admin api key")?,
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
        /// Asynchronously authenticates the request headers and fetches the authentication data and type based on the provided state.
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
        /// Asynchronously authenticates the request using the provided request headers and state, and fetches the authentication data and type.
    ///
    /// # Arguments
    ///
    /// * `request_headers` - The headers of the request.
    /// * `state` - The state containing the merchant account information.
    ///
    /// # Returns
    ///
    /// A `RouterResult` containing a tuple of `AuthenticationData` and `AuthenticationType` if successful, otherwise an error.
    ///
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
        /// Asynchronously authenticates the user based on the provided request headers and fetches the required information. It parses the JWT payload from the request headers and checks if the user is in the blacklist. Then it retrieves the permissions based on the user's role and checks if the user is authorized to access the resource. Finally, it returns a tuple containing an empty value and the authentication type as a result of successful authentication and fetching.
    
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<((), AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if blacklist::check_user_in_blacklist(state, &payload.user_id, payload.exp).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }

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
        /// Asynchronously authenticates the user using the request headers and fetches user information from the provided state.
    /// Returns a tuple containing the user information extracted from the JWT token payload and the authentication type.
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(UserFromToken, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if blacklist::check_user_in_blacklist(state, &payload.user_id, payload.exp).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }

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
        /// Asynchronously authenticates the user using the provided request headers and fetches the required data.
    ///
    /// # Arguments
    ///
    /// * `request_headers` - The headers containing the authentication token
    /// * `state` - The state object containing the necessary data for authentication
    ///
    /// # Returns
    ///
    /// A `RouterResult` containing a tuple with an empty tuple and the `AuthenticationType`
    ///
    /// # Errors
    ///
    /// Returns an error if the JWT token is invalid, the user is blacklisted, or if the user does not have the required permissions.
    ///
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<((), AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if blacklist::check_user_in_blacklist(state, &payload.user_id, payload.exp).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }

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

/// Asynchronously parses the JWT payload from the provided headers and application state.
/// 
/// # Arguments
/// 
/// * `headers` - A reference to the HeaderMap containing the headers of the request.
/// * `state` - A reference to the application state implementing the AppStateInfo trait.
/// 
/// # Returns
/// 
/// Returns a RouterResult containing the parsed JWT payload of type T.
/// 
/// # Generic Types
/// 
/// * `T` - The type into which the JWT payload will be deserialized.
/// * `A` - The type of the application state implementing the AppStateInfo trait.
/// 
/// # Errors
/// 
/// This method may return an error if there is an issue with obtaining the JWT from the authorization header or decoding the JWT payload.
/// 
pub async fn parse_jwt_payload<A, T>(headers: &HeaderMap, state: &A) -> RouterResult<T>
where
    T: serde::de::DeserializeOwned,
    A: AppStateInfo + Sync,
{
    let token = get_jwt_from_authorization_header(headers)?;
    let payload = decode_jwt(token, state).await?;

    Ok(payload)
}

#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationData, A> for JWTAuth
where
    A: AppStateInfo + Sync,
{
        /// Asynchronously authenticates the user and fetches the authentication data and type based on the request headers and state. 
    /// Returns a result containing the authentication data and authentication type if successful, or an error if authentication fails.
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if blacklist::check_user_in_blacklist(state, &payload.user_id, payload.exp).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }

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

pub type AuthenticationDataWithUserId = (AuthenticationData, String);

#[async_trait]
impl<A> AuthenticateAndFetch<AuthenticationDataWithUserId, A> for JWTAuth
where
    A: AppStateInfo + Sync,
{
        /// Asynchronously authenticate the user with the given request headers and fetch the necessary authentication data and type. 
    /// The method parses the JWT payload from the request headers and checks if the user is in the blacklist. If not, it proceeds to get the permissions and perform authorization checks. 
    /// It then retrieves the merchant key store and merchant account using the payload data. Finally, it constructs and returns the authentication data with the user ID and the authentication type.
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(AuthenticationDataWithUserId, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if blacklist::check_user_in_blacklist(state, &payload.user_id, payload.exp).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }

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
            (auth.clone(), payload.user_id.clone()),
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
        /// Asynchronously authenticates the user and fetches the user information from the JWT token
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<(UserFromToken, AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if blacklist::check_user_in_blacklist(state, &payload.user_id, payload.exp).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }

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
        /// Asynchronously authenticates the user using the provided request headers and state, then fetches the authentication type.
    /// 
    /// # Arguments
    /// 
    /// * `request_headers` - The headers from the incoming request.
    /// * `state` - The state object used for authentication.
    /// 
    /// # Returns
    /// 
    /// A Result containing a tuple of an empty payload and the authentication type, or an error if the JWT token is invalid.
    /// 
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<((), AuthenticationType)> {
        let payload = parse_jwt_payload::<A, AuthToken>(request_headers, state).await?;
        if blacklist::check_user_in_blacklist(state, &payload.user_id, payload.exp).await? {
            return Err(errors::ApiErrorResponse::InvalidJwtToken.into());
        }

        Ok(((), AuthenticationType::NoAuth))
    }
}

pub trait ClientSecretFetch {
    fn get_client_secret(&self) -> Option<&String>;
}

impl ClientSecretFetch for payments::PaymentsRequest {
        /// This method returns an optional reference to the client secret string associated with the current instance.
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

/// Retrieves the authentication type and flow based on the provided headers.
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

/// This method checks the client secret and retrieves authentication data based on the provided headers and payload. It returns a tuple containing a trait object implementing AuthenticateAndFetch and an api::AuthFlow enum. The method also has generic constraints for the AppStateInfo trait and the AuthenticateAndFetch trait for ApiKeyAuth and PublishableKeyAuth types.
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

/// Checks if the provided API key is an ephemeral key and validates it against the given customer ID. If the API key is not an ephemeral key, it returns an `ApiKeyAuth` result. If the API key is an ephemeral key, it retrieves the ephemeral key from the database and validates it against the customer ID. If the customer ID is valid, it returns a `MerchantIdAuth` result with the merchant ID from the ephemeral key.
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

/// Checks if the given `HeaderMap` contains the JWT authorization header.
/// 
/// # Arguments
/// * `headers` - A reference to the `HeaderMap` to be checked.
/// 
/// # Returns
/// A boolean value indicating whether the JWT authorization header is present in the `HeaderMap`.
pub fn is_jwt_auth(headers: &HeaderMap) -> bool {
    headers.get(crate::headers::AUTHORIZATION).is_some()
}

static JWT_SECRET: tokio::sync::OnceCell<StrongSecret<String>> = tokio::sync::OnceCell::const_new();

/// Retrieves the JWT secret from the provided secrets and, if the "kms" feature is enabled, decrypts it using the provided KmsClient.
/// If the "kms" feature is enabled, the JWT secret is decrypted using the KmsClient, otherwise the JWT secret is simply cloned from the secrets.
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

/// Asynchronously decodes a JWT token using the provided state information. It retrieves the JWT secret from the state configuration, decodes the token, and returns the decoded claims as a result. If the decoding fails, it returns an error indicating an invalid JWT token.
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

/// Retrieves the API key from the provided headers, if it exists, and returns it as a RouterResult.
pub fn get_api_key(headers: &HeaderMap) -> RouterResult<&str> {
    get_header_value_by_key("api-key".into(), headers)?.get_required_value("api_key")
}

/// Retrieves the header value for the specified key from the given HeaderMap. 
/// If the key is found, the method returns Some(value), otherwise it returns None.
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

/// Retrieves the JWT token from the Authorization header in the provided HeaderMap.
/// If the token is found, it is converted to a string and the "Bearer " prefix is stripped.
/// If the token is not found or cannot be converted to a string, an error is returned.
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

/// Strips the "Bearer " prefix from the provided JWT token and returns the stripped token.
///
/// # Arguments
///
/// * `token` - A string slice representing the JWT token to be stripped.
///
/// # Returns
///
/// Returns a Result containing a reference to the stripped token on success, or an ApiErrorResponse indicating an invalid JWT token.
pub fn strip_jwt_token(token: &str) -> RouterResult<&str> {
    token
        .strip_prefix("Bearer ")
        .ok_or_else(|| errors::ApiErrorResponse::InvalidJwtToken.into())
}

/// This method determines the authentication type based on the presence of JWT token in the request headers. If a JWT token is present, it returns the jwt_auth_type, otherwise it returns the default_auth.
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
static RECON_API_KEY: tokio::sync::OnceCell<StrongSecret<String>> =
    tokio::sync::OnceCell::const_new();

#[cfg(feature = "recon")]
/// Retrieves the recon admin API key either from a KMS-encrypted source or directly from the secrets based on the feature flag. If the "kms" feature is enabled, the KmsClient is used to decrypt the encrypted API key. The retrieved API key is then wrapped in a StrongSecret and returned as a result.
pub async fn get_recon_admin_api_key(
    secrets: &settings::Secrets,
    #[cfg(feature = "kms")] kms_client: &kms::KmsClient,
) -> RouterResult<&'static StrongSecret<String>> {
    RECON_API_KEY
        .get_or_try_init(|| async {
            #[cfg(feature = "kms")]
            let recon_admin_api_key = secrets
                .kms_encrypted_recon_admin_api_key
                .decrypt_inner(kms_client)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to KMS decrypt recon admin API key")?;

            #[cfg(not(feature = "kms"))]
            let recon_admin_api_key = secrets.recon_admin_api_key.clone();

            Ok(StrongSecret::new(recon_admin_api_key))
        })
        .await
}

#[cfg(feature = "recon")]
pub struct ReconAdmin;

#[async_trait]
#[cfg(feature = "recon")]
impl<A> AuthenticateAndFetch<(), A> for ReconAdmin
where
    A: AppStateInfo + Sync,
{
        /// Asynchronously authenticates the request using the provided request headers and fetches necessary data. 
    /// Returns a tuple containing an empty value and the authentication type.
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &A,
    ) -> RouterResult<((), AuthenticationType)> {
        let request_admin_api_key =
            get_api_key(request_headers).change_context(errors::ApiErrorResponse::Unauthorized)?;
        let conf = state.conf();

        let admin_api_key = get_recon_admin_api_key(
            &conf.secrets,
            #[cfg(feature = "kms")]
            kms::get_kms_client(&conf.kms).await,
        )
        .await?;

        if request_admin_api_key != admin_api_key.peek() {
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
    fn get_merchant_id(&self) -> Option<&str> {
        None
    }
}
#[cfg(all(feature = "olap", feature = "recon"))]
#[async_trait]
impl AuthenticateAndFetch<ReconUser, AppState> for ReconJWT {
        /// Asynchronously authenticates the user and fetches their information.
    /// 
    /// This method takes the request headers and the application state as input, and then
    /// parses the JWT payload using the `parse_jwt_payload` function. It returns a tuple
    /// containing the `ReconUser` object with the user ID from the payload, and the
    /// `AuthenticationType` as `NoAuth`.
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &AppState,
    ) -> RouterResult<(ReconUser, AuthenticationType)> {
        let payload = parse_jwt_payload::<AppState, ReconToken>(request_headers, state).await?;

        Ok((
            ReconUser {
                user_id: payload.user_id,
            },
            AuthenticationType::NoAuth,
        ))
    }
}
