use actix_http::header::{HeaderMap, AUTHORIZATION};
use api_models::oidc::{
    AuthCodeData, Jwk, JwksResponse, OidcAuthorizeQuery, OidcDiscoveryResponse, OidcTokenRequest,
    OidcTokenResponse,
};
use base64::Engine;
use common_utils::ext_traits::StringExt;
use error_stack::{report, ResultExt};
use josekit::jws;
use masking::PeekInterface;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    consts::{
        oidc::{
            AUTH_CODE_LENGTH, AUTH_CODE_TTL, CLAIM_AUD, CLAIM_EMAIL, CLAIM_EMAIL_VERIFIED,
            CLAIM_EXP, CLAIM_IAT, CLAIM_ISS, CLAIM_SUB, GRANT_TYPE_AUTHORIZATION_CODE,
            ID_TOKEN_TTL, REDIS_AUTH_CODE_PREFIX, RESPONSE_MODE_QUERY, RESPONSE_TYPE_CODE,
            SCOPE_EMAIL, SCOPE_OPENID, SIGNING_ALG_RS256, SUBJECT_TYPE_PUBLIC,
            TOKEN_AUTH_METHOD_CLIENT_SECRET_BASIC,
        },
        BASE64_ENGINE,
    },
    core::errors::{ApiErrorResponse, RouterResponse},
    routes::app::SessionState,
    services::{api::ApplicationResponse, authentication::UserFromToken},
    utils::user::{get_base_url, get_redis_connection_for_global_tenant},
};
use api_models::payments::RedirectionResponse;
use router_env::tracing;
use url::Url;

/// Build OIDC discovery document
pub async fn get_discovery_document(state: SessionState) -> RouterResponse<OidcDiscoveryResponse> {
    // Backend URL for most endpoints (token, jwks, issuer)
    let backend_base_url = state.conf.user.base_url.clone();

    // Frontend URL only for authorization endpoint (where user gets redirected)
    let frontend_base_url = get_base_url(&state);

    let discovery_response = OidcDiscoveryResponse {
        issuer: backend_base_url.clone(),
        authorization_endpoint: format!("{}/oauth2/authorize", frontend_base_url),
        token_endpoint: format!("{}/oauth2/token", backend_base_url),
        jwks_uri: format!("{}/oauth2/jwks", backend_base_url),
        response_types_supported: vec![RESPONSE_TYPE_CODE.to_string()],
        response_modes_supported: vec![RESPONSE_MODE_QUERY.to_string()],
        subject_types_supported: vec![SUBJECT_TYPE_PUBLIC.to_string()],
        id_token_signing_alg_values_supported: vec![SIGNING_ALG_RS256.to_string()],
        grant_types_supported: vec![GRANT_TYPE_AUTHORIZATION_CODE.to_string()],
        scopes_supported: vec![SCOPE_OPENID.to_string(), SCOPE_EMAIL.to_string()],
        token_endpoint_auth_methods_supported: vec![
            TOKEN_AUTH_METHOD_CLIENT_SECRET_BASIC.to_string()
        ],
        claims_supported: vec![
            CLAIM_AUD.to_string(),
            CLAIM_EMAIL.to_string(),
            CLAIM_EMAIL_VERIFIED.to_string(),
            CLAIM_EXP.to_string(),
            CLAIM_IAT.to_string(),
            CLAIM_ISS.to_string(),
            CLAIM_SUB.to_string(),
        ],
    };

    Ok(ApplicationResponse::Json(discovery_response))
}

/// Build JWKS response with public keys (all keys for token validation)
pub async fn get_jwks(state: SessionState) -> RouterResponse<JwksResponse> {
    let oidc_keys = state.conf.oidc.get_all_keys();

    let mut jwks = Vec::new();

    for key_config in oidc_keys {
        let private_key_pem = key_config.private_key.peek();

        let (n, e) = common_utils::crypto::extract_rsa_public_key_components(private_key_pem)
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to extract public key from private key")?;

        let jwk = Jwk {
            kty: "RSA".to_string(),
            kid: key_config.kid.clone(),
            key_use: "sig".to_string(),
            alg: SIGNING_ALG_RS256.to_string(),
            n,
            e,
        };

        jwks.push(jwk);
    }

    let jwks_response = JwksResponse { keys: jwks };

    Ok(ApplicationResponse::Json(jwks_response))
}

pub fn validate_authorize_params(
    payload: &OidcAuthorizeQuery,
    state: &SessionState,
) -> error_stack::Result<(), ApiErrorResponse> {
    if payload.response_type != RESPONSE_TYPE_CODE {
        return Err(report!(ApiErrorResponse::OidcAuthorizationError {
            message: "response_type is not supported by the authorization server".to_string(),
        }));
    }
    if !payload.scope.split_whitespace().any(|s| s == SCOPE_OPENID) {
        return Err(report!(ApiErrorResponse::OidcAuthorizationError {
            message: "invalid_scope: The requested scope is invalid, unknown, or malformed"
                .to_string(),
        }));
    }
    if payload.state.is_none() {
        return Err(report!(ApiErrorResponse::OidcAuthorizationError {
            message: "state parameter is required".to_string(),
        }));
    }
    if payload.nonce.is_none() {
        return Err(report!(ApiErrorResponse::OidcAuthorizationError {
            message: "nonce parameter is required".to_string(),
        }));
    }
    let client = state
        .conf
        .oidc
        .get_client(&payload.client_id)
        .ok_or_else(|| {
            report!(ApiErrorResponse::OidcAuthorizationError {
                message: "unauthorized_client: Unknown or unregistered client_id".to_string(),
            })
        })?;
    if client.redirect_uri != payload.redirect_uri {
        return Err(report!(ApiErrorResponse::OidcAuthorizationError {
            message: "invalid_request: The redirect_uri provided does not match a registered redirect_uri for this client".to_string(),
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

    let user_email = user_from_db.email.peek().to_string();

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
        .serialize_and_set_key_with_expiry(&redis_key.into(), &auth_code_data, AUTH_CODE_TTL)
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
    let mut url = Url::parse(redirect_uri)
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to parse redirect_uri")?;

    url.query_pairs_mut()
        .append_pair("code", auth_code)
        .append_pair("state", state);

    Ok(url.to_string())
}

pub async fn process_authorize_request(
    state: SessionState,
    payload: OidcAuthorizeQuery,
    user: Option<UserFromToken>,
) -> RouterResponse<()> {
    // Validate all parameters
    validate_authorize_params(&payload, &state)?;
    // TODO: TO handle users who are not already logged in
    let user = match user {
        None => {
            return Err(report!(ApiErrorResponse::OidcAuthorizationError {
                message: "login_required: No active session found. Please log in to continue."
                    .to_string(),
            }));
        }
        Some(user) => user,
    };

    let auth_code = generate_and_store_authorization_code(&state, &user.user_id, &payload).await?;

    let state_param = payload.state.as_ref().ok_or_else(|| {
        report!(ApiErrorResponse::OidcAuthorizationError {
            message: "state parameter is required".to_string(),
        })
    })?;

    let redirect_url = build_oidc_redirect_url(&payload.redirect_uri, &auth_code, state_param)?;

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

pub fn parse_basic_auth(
    headers: &HeaderMap,
) -> error_stack::Result<(String, String), ApiErrorResponse> {
    let auth_header = headers
        .get(AUTHORIZATION)
        .ok_or_else(|| {
            report!(ApiErrorResponse::OidcTokenError {
                message: "invalid_client: Missing Authorization header".to_string(),
            })
        })?
        .to_str()
        .change_context(ApiErrorResponse::OidcTokenError {
            message: "invalid_client: Invalid Authorization header encoding".to_string(),
        })
        .attach_printable("Failed to convert Authorization header to string")?;

    // Check for "Basic " prefix
    let base64_credentials = auth_header.strip_prefix("Basic ").ok_or_else(|| {
        report!(ApiErrorResponse::OidcTokenError {
            message: "invalid_client: Invalid Authorization header format. Expected Basic auth."
                .to_string(),
        })
    })?;

    // Decode base64
    let decoded = BASE64_ENGINE.decode(base64_credentials).change_context(
        ApiErrorResponse::OidcTokenError {
            message: "invalid_client: Failed to decode Basic auth credentials".to_string(),
        },
    )?;

    // Convert to UTF-8 string
    let credentials =
        String::from_utf8(decoded).change_context(ApiErrorResponse::OidcTokenError {
            message: "invalid_client: Invalid credentials encoding".to_string(),
        })?;

    let (client_id, client_secret) = credentials.split_once(':').ok_or_else(|| {
        report!(ApiErrorResponse::OidcTokenError {
            message: "invalid_client: Invalid credentials format. Expected client_id:client_secret"
                .to_string(),
        })
    })?;
    let client_id = client_id.trim();
    let client_secret = client_secret.trim();

    if client_id.is_empty() {
        return Err(report!(ApiErrorResponse::OidcTokenError {
            message: "invalid_client: client_id cannot be empty".to_string(),
        }));
    }

    if client_secret.is_empty() {
        return Err(report!(ApiErrorResponse::OidcTokenError {
            message: "invalid_client: client_secret cannot be empty".to_string(),
        }));
    }

    Ok((client_id.to_string(), client_secret.to_string()))
}

fn validate_oidc_client_credentials(
    state: &SessionState,
    basic_client_id: &str,
    basic_client_secret: &str,
    request_client_id: &str,
    redirect_uri: &str,
) -> error_stack::Result<(), ApiErrorResponse> {
    let oidc_client = state.conf.oidc.get_client(basic_client_id).ok_or_else(|| {
        report!(ApiErrorResponse::OidcTokenError {
            message: "invalid_client: Unknown client_id".to_string(),
        })
    })?;
    if basic_client_secret != oidc_client.client_secret.peek() {
        return Err(report!(ApiErrorResponse::OidcTokenError {
            message: "invalid_client: Invalid client_secret".to_string(),
        }));
    }
    if basic_client_id != request_client_id {
        return Err(report!(ApiErrorResponse::OidcTokenError {
            message:
                "invalid_request: client_id mismatch between Authorization header and request body"
                    .to_string(),
        }));
    }
    if oidc_client.redirect_uri != redirect_uri {
        return Err(report!(ApiErrorResponse::OidcTokenError {
            message: "invalid_request: The redirect_uri provided does not match a registered redirect_uri for this client".to_string(),
        }));
    }

    Ok(())
}

fn validate_token_request_params(
    grant_type: &str,
    redirect_uri: &str,
) -> error_stack::Result<(), ApiErrorResponse> {
    if grant_type != GRANT_TYPE_AUTHORIZATION_CODE {
        return Err(report!(ApiErrorResponse::OidcTokenError {
            message: format!(
                "unsupported_grant_type: grant_type must be '{}'",
                GRANT_TYPE_AUTHORIZATION_CODE
            ),
        }));
    }
    if redirect_uri.is_empty() {
        return Err(report!(ApiErrorResponse::OidcTokenError {
            message: "invalid_request: redirect_uri is required".to_string(),
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
                message: "invalid_grant: The provided authorization code is invalid or has expired"
                    .to_string(),
            })
        })?;

    let auth_code_data: AuthCodeData = auth_code_data_string
        .parse_struct("AuthCodeData")
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to deserialize authorization code data")?;

    if auth_code_data.client_id != client_id {
        return Err(report!(ApiErrorResponse::OidcTokenError {
            message: "invalid_grant: client_id mismatch with authorization code".to_string(),
        }));
    }
    if auth_code_data.redirect_uri != redirect_uri {
        return Err(report!(ApiErrorResponse::OidcTokenError {
            message: "invalid_grant: redirect_uri mismatch with authorization code".to_string(),
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
    #[serde(rename = "iss")]
    issuer: String,
    #[serde(rename = "sub")]
    subject: String,
    #[serde(rename = "aud")]
    audience: String,
    #[serde(rename = "iat")]
    issued_at: u64,
    #[serde(rename = "exp")]
    expires_at: u64,
    #[serde(rename = "email")]
    email: String,
    #[serde(rename = "email_verified")]
    email_verified: bool,
    #[serde(rename = "nonce", skip_serializing_if = "Option::is_none")]
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

    let exp = now.checked_add(ID_TOKEN_TTL).ok_or_else(|| {
        report!(ApiErrorResponse::InternalServerError)
            .attach_printable("Token expiration time overflow")
    })?;

    let claims = IdTokenClaims {
        issuer: state.conf.user.base_url.clone(),
        subject: auth_code_data.sub.clone(),
        audience: auth_code_data.client_id.clone(),
        issued_at: now,
        expires_at: exp,
        email: auth_code_data.email.clone(),
        email_verified: true,
        nonce: auth_code_data.nonce.clone(),
    };

    let payload_bytes = serde_json::to_vec(&claims)
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialize ID token claims")?;

    let alg = jws::RS256;
    let mut header = jws::JwsHeader::new();
    header.set_key_id(&signing_key.kid);

    let signer = alg
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
    headers: HeaderMap,
) -> RouterResponse<OidcTokenResponse> {
    let (basic_client_id, basic_client_secret) = parse_basic_auth(&headers)?;
    validate_oidc_client_credentials(
        &state,
        &basic_client_id,
        &basic_client_secret,
        &payload.client_id,
        &payload.redirect_uri,
    )?;

    validate_token_request_params(&payload.grant_type, &payload.redirect_uri)?;

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
        token_type: "Bearer".to_string(),
        expires_in: ID_TOKEN_TTL,
    }))
}
