use std::time::{Duration, SystemTime, UNIX_EPOCH};

use api_models::{
    oidc::{
        Jwk, JwksResponse, KeyType, KeyUse, OidcAuthorizationError, OidcAuthorizeQuery,
        OidcDiscoveryResponse, OidcTokenError, OidcTokenRequest, OidcTokenResponse, Scope,
        SigningAlgorithm,
    },
    payments::RedirectionResponse,
};
use common_utils::pii;
use error_stack::{report, ResultExt};
use masking::PeekInterface;
use once_cell::sync::OnceCell;
use router_env::logger;

use crate::{
    consts::oidc::{
        ACCESS_TOKEN_TTL_IN_SECS, AUTH_CODE_LENGTH, ID_TOKEN_TTL_IN_SECS, TOKEN_TYPE_BEARER,
    },
    core::errors::{ApiErrorResponse, RouterResponse},
    routes::app::SessionState,
    services::{api::ApplicationResponse, authentication::UserFromToken, jwt},
    types::domain::user::oidc::{AccessTokenClaims, AuthCodeData, IdTokenClaims},
    utils::{oidc as oidc_utils, user as user_utils},
};

/// Build OIDC discovery document
pub async fn get_discovery_document(state: SessionState) -> RouterResponse<OidcDiscoveryResponse> {
    let backend_base_url = state.tenant.base_url.clone();
    let control_center_url = user_utils::get_base_url(&state);

    Ok(ApplicationResponse::Json(OidcDiscoveryResponse::new(
        backend_base_url,
        control_center_url.into(),
    )))
}

static CACHED_JWKS: OnceCell<JwksResponse> = OnceCell::new();
/// Build JWKS response with public keys (all keys for token validation)
pub async fn get_jwks(state: SessionState) -> RouterResponse<&'static JwksResponse> {
    CACHED_JWKS
        .get_or_try_init(|| {
            let oidc_keys = state.conf.oidc.get_inner().get_all_keys();
            let mut keys = Vec::with_capacity(oidc_keys.len());

            for key_config in oidc_keys {
                let (n, e) = common_utils::crypto::extract_rsa_public_key_components(
                    &key_config.private_key,
                )
                .change_context(ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to extract public key from private key")?;

                let jwk = Jwk {
                    kty: KeyType::Rsa,
                    kid: key_config.kid.clone(),
                    key_use: KeyUse::Sig,
                    alg: SigningAlgorithm::Rs256,
                    n,
                    e,
                };

                keys.push(jwk);
            }

            Ok::<_, error_stack::Report<ApiErrorResponse>>(JwksResponse { keys })
        })
        .map(ApplicationResponse::Json)
}

fn validate_authorize_params(
    payload: &OidcAuthorizeQuery,
    state: &SessionState,
) -> error_stack::Result<(), ApiErrorResponse> {
    if !payload.scope.contains(&Scope::Openid) {
        let error = OidcAuthorizationError::InvalidScope;
        return Err(report!(ApiErrorResponse::OidcAuthorizationError {
            error,
            description: error.description().into(),
        }));
    }
    let client = state
        .conf
        .oidc
        .get_inner()
        .get_client(&payload.client_id)
        .ok_or_else(|| {
            let error = OidcAuthorizationError::UnauthorizedClient;
            report!(ApiErrorResponse::OidcAuthorizationError {
                error,
                description: error.description().into(),
            })
        })?;

    if !oidc_utils::validate_redirect_uri_match(&client.redirect_uri, &payload.redirect_uri) {
        let error = OidcAuthorizationError::InvalidRequest;
        return Err(report!(ApiErrorResponse::OidcAuthorizationError {
            error,
            description: error.description().into(),
        }));
    }

    Ok(())
}

async fn generate_and_store_authorization_code(
    state: &SessionState,
    user_id: &str,
    payload: &OidcAuthorizeQuery,
) -> error_stack::Result<String, ApiErrorResponse> {
    let user_from_db = state
        .global_store
        .find_user_by_id(user_id)
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch user from database")?;

    let user_email = pii::Email::try_from(user_from_db.email.peek().to_string())
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to parse user email")?;

    let auth_code =
        common_utils::crypto::generate_cryptographically_secure_random_string(AUTH_CODE_LENGTH);

    let auth_code_data = AuthCodeData {
        sub: user_id.to_string(),
        client_id: payload.client_id.clone(),
        redirect_uri: payload.redirect_uri.clone(),
        scope: payload.scope.clone(),
        nonce: payload.nonce.clone(),
        email: user_email,
        is_verified: user_from_db.is_verified,
    };

    oidc_utils::set_auth_code_in_redis(state, &auth_code, &auth_code_data).await?;

    Ok(auth_code)
}

pub async fn process_authorize_request(
    state: SessionState,
    payload: OidcAuthorizeQuery,
    user: Option<UserFromToken>,
) -> RouterResponse<()> {
    // Validate all parameters
    validate_authorize_params(&payload, &state)?;
    // TODO: Handle users who are not already logged in
    let user = user.ok_or_else(|| {
        let error = OidcAuthorizationError::AccessDenied;
        report!(ApiErrorResponse::OidcAuthorizationError {
            error,
            description: error.description().into(),
        })
    })?;

    let auth_code = generate_and_store_authorization_code(&state, &user.user_id, &payload).await?;

    let redirect_url =
        oidc_utils::build_oidc_redirect_url(&payload.redirect_uri, &auth_code, &payload.state)?;

    #[cfg(feature = "v1")]
    {
        Ok(ApplicationResponse::JsonForRedirection(
            RedirectionResponse {
                headers: Vec::new(),
                return_url: String::new(),
                http_method: String::new(),
                params: Vec::new(),
                return_url_with_query_params: redirect_url,
            },
        ))
    }

    #[cfg(feature = "v2")]
    {
        Ok(ApplicationResponse::JsonForRedirection(
            RedirectionResponse {
                return_url_with_query_params: redirect_url,
            },
        ))
    }
}

fn validate_token_request(
    state: &SessionState,
    client_id: &str,
    redirect_uri: &str,
) -> error_stack::Result<(), ApiErrorResponse> {
    if redirect_uri.trim().is_empty() {
        let error = OidcTokenError::InvalidRequest;
        return Err(report!(ApiErrorResponse::OidcTokenError {
            error,
            description: error.description().into(),
        }));
    }

    let registered_client = state
        .conf
        .oidc
        .get_inner()
        .get_client(client_id)
        .ok_or_else(|| {
            let error = OidcTokenError::InvalidClient;
            report!(ApiErrorResponse::OidcTokenError {
                error,
                description: error.description().into(),
            })
        })?;

    if !oidc_utils::validate_redirect_uri_match(&registered_client.redirect_uri, redirect_uri) {
        let error = OidcTokenError::InvalidRequest;
        return Err(report!(ApiErrorResponse::OidcTokenError {
            error,
            description: error.description().into(),
        }));
    }

    Ok(())
}

async fn validate_and_consume_authorization_code(
    state: &SessionState,
    code: &str,
    client_id: &str,
    redirect_uri: &str,
) -> error_stack::Result<AuthCodeData, ApiErrorResponse> {
    let auth_code_data = oidc_utils::get_auth_code_from_redis(state, code).await?;

    if !oidc_utils::validate_client_id_match(&auth_code_data.client_id, client_id) {
        let error = OidcTokenError::InvalidGrant;
        return Err(report!(ApiErrorResponse::OidcTokenError {
            error,
            description: error.description().into(),
        }));
    }

    if !oidc_utils::validate_redirect_uri_match(&auth_code_data.redirect_uri, redirect_uri) {
        let error = OidcTokenError::InvalidGrant;
        return Err(report!(ApiErrorResponse::OidcTokenError {
            error,
            description: error.description().into(),
        }));
    }

    if let Err(err) = oidc_utils::delete_auth_code_from_redis(state, code).await {
        logger::warn!(
            error = ?err,
            "Failed to delete authorization code from Redis after consumption"
        );
    }

    Ok(auth_code_data)
}

async fn generate_id_token(
    state: &SessionState,
    auth_code_data: &AuthCodeData,
) -> error_stack::Result<String, ApiErrorResponse> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .change_context(ApiErrorResponse::InternalServerError)?
        .as_secs();
    let exp_duration = Duration::from_secs(ID_TOKEN_TTL_IN_SECS);
    let exp = jwt::generate_exp(exp_duration)
        .change_context(ApiErrorResponse::InternalServerError)?
        .as_secs();

    let include_email_claims = auth_code_data.scope.contains(&Scope::Email);
    let include_profile_claims = auth_code_data.scope.contains(&Scope::Profile);

    let claims = IdTokenClaims {
        iss: state.tenant.base_url.clone(),
        sub: auth_code_data.sub.clone(),
        aud: auth_code_data.client_id.clone(),
        iat: now,
        exp,
        email: include_email_claims.then_some(auth_code_data.email.clone()),
        email_verified: include_email_claims.then_some(auth_code_data.is_verified),
        preferred_username: include_profile_claims
            .then_some(auth_code_data.email.peek().to_string()),
        nonce: auth_code_data.nonce.clone(),
    };

    oidc_utils::sign_oidc_token(state, claims, "ID token").await
}

async fn generate_access_token(
    state: &SessionState,
    auth_code_data: &AuthCodeData,
) -> error_stack::Result<String, ApiErrorResponse> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .change_context(ApiErrorResponse::InternalServerError)?
        .as_secs();
    let exp_duration = Duration::from_secs(ACCESS_TOKEN_TTL_IN_SECS);
    let exp = jwt::generate_exp(exp_duration)
        .change_context(ApiErrorResponse::InternalServerError)?
        .as_secs();

    let include_profile_claims = auth_code_data.scope.contains(&Scope::Profile);

    let claims = AccessTokenClaims {
        iss: state.tenant.base_url.clone(),
        sub: auth_code_data.sub.clone(),
        aud: auth_code_data.client_id.clone(),
        iat: now,
        exp,
        email: auth_code_data.email.clone(),
        preferred_username: include_profile_claims
            .then_some(auth_code_data.email.peek().to_string()),
        scope: auth_code_data.scope.clone(),
    };

    oidc_utils::sign_oidc_token(state, claims, "access token").await
}

pub async fn process_token_request(
    state: SessionState,
    payload: OidcTokenRequest,
    client_id: String,
) -> RouterResponse<OidcTokenResponse> {
    validate_token_request(&state, &client_id, &payload.redirect_uri)?;

    let auth_code_data = validate_and_consume_authorization_code(
        &state,
        &payload.code,
        &client_id,
        &payload.redirect_uri,
    )
    .await?;

    let access_token = generate_access_token(&state, &auth_code_data).await?;
    let id_token = generate_id_token(&state, &auth_code_data).await?;

    Ok(ApplicationResponse::Json(OidcTokenResponse {
        access_token: access_token.into(),
        id_token: id_token.into(),
        token_type: TOKEN_TYPE_BEARER.to_string(),
        expires_in: ACCESS_TOKEN_TTL_IN_SECS,
    }))
}
