use std::time::{SystemTime, UNIX_EPOCH};

use api_models::{
    oidc::{
        AuthCodeData, Jwk, JwksResponse, KeyType, KeyUse, OidcAuthorizationError,
        OidcAuthorizeQuery, OidcDiscoveryResponse, OidcTokenError, OidcTokenRequest,
        OidcTokenResponse, Scope, SigningAlgorithm,
    },
    payments::RedirectionResponse,
};
use common_utils::{ext_traits::StringExt, pii};
use error_stack::{report, ResultExt};
use josekit::jws;
use masking::PeekInterface;
use once_cell::sync::OnceCell;
use router_env::tracing;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    consts::oidc::{
        AUTH_CODE_LENGTH, AUTH_CODE_TTL_IN_SECS, ID_TOKEN_TTL_IN_SECS, REDIS_AUTH_CODE_PREFIX,
        TOKEN_TYPE_BEARER,
    },
    core::errors::{ApiErrorResponse, RouterResponse},
    routes::app::SessionState,
    services::{api::ApplicationResponse, authentication::UserFromToken},
    utils::user::{get_base_url, get_redis_connection_for_global_tenant},
};

/// Build OIDC discovery document
pub async fn get_discovery_document(state: SessionState) -> RouterResponse<OidcDiscoveryResponse> {
    let backend_base_url = state.tenant.base_url.clone();
    let control_center_url = get_base_url(&state);

    Ok(ApplicationResponse::Json(OidcDiscoveryResponse::new(
        backend_base_url,
        control_center_url.into(),
    )))
}

static CACHED_JWKS: OnceCell<JwksResponse> = OnceCell::new();
/// Build JWKS response with public keys (all keys for token validation)
pub async fn get_jwks(state: SessionState) -> RouterResponse<JwksResponse> {
    let jwks_response = CACHED_JWKS.get_or_try_init(|| {
        let oidc_keys = state.conf.oidc.get_all_keys();
        let mut keys = Vec::new();

        for key_config in oidc_keys {
            let (n, e) =
                common_utils::crypto::extract_rsa_public_key_components(&key_config.private_key)
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
    })?;

    Ok(ApplicationResponse::Json(jwks_response.clone()))
}

fn validate_client_id_match(registered_client_id: &str, provided_client_id: &str) -> bool {
    registered_client_id.trim() == provided_client_id.trim()
}

fn validate_redirect_uri_match(registered_uri: &str, provided_uri: &str) -> bool {
    match (
        Url::parse(registered_uri.trim()),
        Url::parse(provided_uri.trim()),
    ) {
        (Ok(registered_url), Ok(provided_url)) => registered_url == provided_url,
        _ => registered_uri.trim() == provided_uri.trim(),
    }
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
        .get_client(&payload.client_id)
        .ok_or_else(|| {
            report!(ApiErrorResponse::OidcAuthorizationError {
                error: OidcAuthorizationError::UnauthorizedClient,
                description: "Unknown client_id".into(),
            })
        })?;

    if !validate_redirect_uri_match(&client.redirect_uri, &payload.redirect_uri) {
        return Err(report!(ApiErrorResponse::OidcAuthorizationError {
            error: OidcAuthorizationError::InvalidRequest,
            description: "redirect_uri mismatch".into(),
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
    };

    let redis_conn = get_redis_connection_for_global_tenant(state)
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get Redis connection")?;

    let redis_key = format!("{}{}", REDIS_AUTH_CODE_PREFIX, auth_code);
    redis_conn
        .serialize_and_set_key_with_expiry(
            &redis_key.as_str().into(),
            &auth_code_data,
            AUTH_CODE_TTL_IN_SECS,
        )
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to store authorization code in Redis")?;

    Ok(auth_code)
}

pub fn build_oidc_redirect_url(
    redirect_uri: &str,
    auth_code: &str,
    state: &str,
) -> error_stack::Result<String, ApiErrorResponse> {
    let base_url = Url::parse(redirect_uri).map_err(|_| {
        report!(ApiErrorResponse::InternalServerError)
            .attach_printable("Invalid redirect_uri in OIDC authorize request")
    })?;

    let url = Url::parse_with_params(base_url.as_str(), &[("code", auth_code), ("state", state)])
        .map_err(|_| {
        report!(ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to build redirect URL with query parameters")
    })?;

    Ok(url.to_string())
}

#[cfg(feature = "v1")]
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

    let redirect_url = build_oidc_redirect_url(&payload.redirect_uri, &auth_code, &payload.state)?;

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

fn validate_token_request(
    state: &SessionState,
    authenticated_client_id: &str,
    request_client_id: &str,
    redirect_uri: &str,
) -> error_stack::Result<(), ApiErrorResponse> {
    if redirect_uri.trim().is_empty() {
        return Err(report!(ApiErrorResponse::OidcTokenError {
            error: OidcTokenError::InvalidRequest,
            description: "redirect_uri is required".into(),
        }));
    }

    if !validate_client_id_match(authenticated_client_id, request_client_id) {
        return Err(report!(ApiErrorResponse::OidcTokenError {
            error: OidcTokenError::InvalidRequest,
            description: "client_id mismatch".into(),
        }));
    }

    let registered_client = state
        .conf
        .oidc
        .get_client(request_client_id)
        .ok_or_else(|| {
            report!(ApiErrorResponse::OidcTokenError {
                error: OidcTokenError::InvalidClient,
                description: "Unknown client_id".into(),
            })
        })?;

    if !validate_redirect_uri_match(&registered_client.redirect_uri, redirect_uri) {
        return Err(report!(ApiErrorResponse::OidcTokenError {
            error: OidcTokenError::InvalidRequest,
            description: "redirect_uri mismatch".into(),
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
    let redis_conn = get_redis_connection_for_global_tenant(state)
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get Redis connection")?;

    let redis_key = format!("{}{}", REDIS_AUTH_CODE_PREFIX, code);
    let auth_code_data_string = redis_conn
        .get_key::<Option<String>>(&redis_key.as_str().into())
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch authorization code from Redis")?
        .ok_or_else(|| {
            report!(ApiErrorResponse::OidcTokenError {
                error: OidcTokenError::InvalidGrant,
                description: "Invalid or expired authorization code".into(),
            })
        })?;

    let auth_code_data: AuthCodeData = auth_code_data_string
        .parse_struct("AuthCodeData")
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to deserialize authorization code data")?;

    if !validate_client_id_match(&auth_code_data.client_id, client_id) {
        return Err(report!(ApiErrorResponse::OidcTokenError {
            error: OidcTokenError::InvalidGrant,
            description: "client_id mismatch".into(),
        }));
    }

    if !validate_redirect_uri_match(&auth_code_data.redirect_uri, redirect_uri) {
        return Err(report!(ApiErrorResponse::OidcTokenError {
            error: OidcTokenError::InvalidGrant,
            description: "redirect_uri mismatch".into(),
        }));
    }

    if let Err(err) = redis_conn.delete_key(&redis_key.as_str().into()).await {
        tracing::error!("Failed to delete authorization code from Redis: {:?}", err);
    }

    Ok(auth_code_data)
}

/// ID Token Claims structure
#[derive(Debug, Serialize, Deserialize)]
struct IdTokenClaims {
    iss: String,
    sub: String,
    aud: String,
    iat: u64,
    exp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<pii::Email>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email_verified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nonce: Option<String>,
}

async fn generate_id_token(
    state: &SessionState,
    auth_code_data: &AuthCodeData,
) -> error_stack::Result<String, ApiErrorResponse> {
    let signing_key = state.conf.oidc.get_signing_key().ok_or_else(|| {
        report!(ApiErrorResponse::InternalServerError)
            .attach_printable("No signing key configured for OIDC")
    })?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get current time")?
        .as_secs();

    let exp = now.checked_add(ID_TOKEN_TTL_IN_SECS).ok_or_else(|| {
        report!(ApiErrorResponse::InternalServerError)
            .attach_printable("Token expiration time overflow")
    })?;

    let include_email_claims = auth_code_data.scope.contains(&Scope::Email);

    let claims = IdTokenClaims {
        iss: state.conf.user.base_url.clone(),
        sub: auth_code_data.sub.clone(),
        aud: auth_code_data.client_id.clone(),
        iat: now,
        exp,
        email: include_email_claims.then_some(auth_code_data.email.clone()),
        email_verified: include_email_claims.then_some(true),
        nonce: auth_code_data.nonce.clone(),
    };

    let payload_bytes = serde_json::to_vec(&claims)
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialize ID token claims")?;

    let signing_algorithm = jws::RS256;
    let mut header = jws::JwsHeader::new();
    header.set_key_id(&signing_key.kid);

    let signer = signing_algorithm
        .signer_from_pem(signing_key.private_key.peek().as_bytes())
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to create JWT signer from private key")?;

    let id_token = jws::serialize_compact(&payload_bytes, &header, &signer)
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to sign ID token")?;

    Ok(id_token)
}

pub async fn process_token_request(
    state: SessionState,
    payload: OidcTokenRequest,
    client_id: String,
) -> RouterResponse<OidcTokenResponse> {
    validate_token_request(
        &state,
        &client_id,
        &payload.client_id,
        &payload.redirect_uri,
    )?;

    let auth_code_data = validate_and_consume_authorization_code(
        &state,
        &payload.code,
        &payload.client_id,
        &payload.redirect_uri,
    )
    .await?;

    let id_token = generate_id_token(&state, &auth_code_data).await?;

    Ok(ApplicationResponse::Json(OidcTokenResponse {
        id_token,
        token_type: TOKEN_TYPE_BEARER.to_string(),
        expires_in: ID_TOKEN_TTL_IN_SECS,
    }))
}
