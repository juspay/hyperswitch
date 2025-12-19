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

use crate::{
    consts::oidc::{
        ACCESS_TOKEN_TTL_IN_SECS, AUTH_CODE_LENGTH, ID_TOKEN_TTL_IN_SECS, TOKEN_TYPE_BEARER,
    },
    core::errors::{ApiErrorResponse, RouterResponse},
    routes::app::SessionState,
    services::{api::ApplicationResponse, authentication::UserFromToken},
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

pub fn validate_authorize_params(
    payload: &OidcAuthorizeQuery,
    state: &SessionState,
) -> error_stack::Result<(), ApiErrorResponse> {
    if !payload.scope.contains(&Scope::Openid) {
        return Err(report!(ApiErrorResponse::OidcAuthorizationError {
            error: OidcAuthorizationError::InvalidScope,
            description: "Missing required 'openid' scope".into(),
        }));
    }
    let client = state
        .conf
        .oidc
        .get_inner()
        .get_client(&payload.client_id)
        .ok_or_else(|| {
            report!(ApiErrorResponse::OidcAuthorizationError {
                error: OidcAuthorizationError::UnauthorizedClient,
                description: "Unknown client_id".into(),
            })
        })?;

    if !oidc_utils::validate_redirect_uri_match(&client.redirect_uri, &payload.redirect_uri) {
        return Err(report!(ApiErrorResponse::OidcAuthorizationError {
            error: OidcAuthorizationError::InvalidRequest,
            description: "redirect_uri mismatch".into(),
        }));
    }

    Ok(())
}

pub async fn generate_and_store_authorization_code(
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
    let user = match user {
        None => {
            return Err(report!(ApiErrorResponse::OidcAuthorizationError {
                error: OidcAuthorizationError::AccessDenied,
                description: "User not authenticated".into(),
            }));
        }
        Some(user) => user,
    };

    let auth_code = generate_and_store_authorization_code(&state, &user.user_id, &payload).await?;

    let redirect_url =
        oidc_utils::build_oidc_redirect_url(&payload.redirect_uri, &auth_code, &payload.state)?;

    #[cfg(feature = "v1")]
    {
        Ok(ApplicationResponse::JsonForRedirection(
            RedirectionResponse {
                headers: Vec::with_capacity(0),
                return_url: String::new(),
                http_method: String::new(),
                params: Vec::with_capacity(0),
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

pub fn validate_token_request(
    state: &SessionState,
    client_id: &str,
    redirect_uri: &str,
) -> error_stack::Result<(), ApiErrorResponse> {
    if redirect_uri.trim().is_empty() {
        return Err(report!(ApiErrorResponse::OidcTokenError {
            error: OidcTokenError::InvalidRequest,
            description: "redirect_uri is required".into(),
        }));
    }

    let registered_client = state
        .conf
        .oidc
        .get_inner()
        .get_client(client_id)
        .ok_or_else(|| {
            report!(ApiErrorResponse::OidcTokenError {
                error: OidcTokenError::InvalidClient,
                description: "Unknown client_id".into(),
            })
        })?;

    if !oidc_utils::validate_redirect_uri_match(&registered_client.redirect_uri, redirect_uri) {
        return Err(report!(ApiErrorResponse::OidcTokenError {
            error: OidcTokenError::InvalidRequest,
            description: "redirect_uri mismatch".into(),
        }));
    }

    Ok(())
}

pub async fn validate_and_consume_authorization_code(
    state: &SessionState,
    code: &str,
    client_id: &str,
    redirect_uri: &str,
) -> error_stack::Result<AuthCodeData, ApiErrorResponse> {
    let auth_code_data = oidc_utils::get_auth_code_from_redis(state, code).await?;

    if !oidc_utils::validate_client_id_match(&auth_code_data.client_id, client_id) {
        return Err(report!(ApiErrorResponse::OidcTokenError {
            error: OidcTokenError::InvalidGrant,
            description: "client_id mismatch".into(),
        }));
    }

    if !oidc_utils::validate_redirect_uri_match(&auth_code_data.redirect_uri, redirect_uri) {
        return Err(report!(ApiErrorResponse::OidcTokenError {
            error: OidcTokenError::InvalidGrant,
            description: "redirect_uri mismatch".into(),
        }));
    }

    oidc_utils::delete_auth_code_from_redis(state, code).await?;

    Ok(auth_code_data)
}

pub async fn generate_id_token(
    state: &SessionState,
    auth_code_data: &AuthCodeData,
) -> error_stack::Result<String, ApiErrorResponse> {
    let (now, exp) = oidc_utils::generate_oidc_now_and_exp(ID_TOKEN_TTL_IN_SECS)?;

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

pub async fn generate_access_token(
    state: &SessionState,
    auth_code_data: &AuthCodeData,
) -> error_stack::Result<String, ApiErrorResponse> {
    let (now, exp) = oidc_utils::generate_oidc_now_and_exp(ACCESS_TOKEN_TTL_IN_SECS)?;

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
