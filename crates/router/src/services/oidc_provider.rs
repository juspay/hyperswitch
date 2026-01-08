use std::time::{Duration, SystemTime, UNIX_EPOCH};

use api_models::{
    oidc::{
        Jwk, JwksResponse, KeyType, KeyUse, OidcAuthorizeQuery, OidcDiscoveryResponse,
        OidcTokenRequest, OidcTokenResponse, Scope, SigningAlgorithm,
    },
    payments::RedirectionResponse,
};
use error_stack::{report, ResultExt};
use masking::PeekInterface;
use once_cell::sync::OnceCell;
use router_env::logger;

use crate::{
    consts::oidc::{
        ACCESS_TOKEN_TTL_IN_SECS, AUTH_CODE_LENGTH, ID_TOKEN_TTL_IN_SECS, TOKEN_TYPE_BEARER,
    },
    core::errors::oidc::{OidcErrors, OidcResponse, OidcResult},
    routes::app::SessionState,
    services::{api::ApplicationResponse, authentication::UserFromToken, jwt},
    types::domain::user::oidc::{AccessTokenClaims, AuthCodeData, IdTokenClaims},
    utils::{oidc as oidc_utils, user as user_utils},
};

/// Build OIDC discovery document
pub async fn get_discovery_document(state: SessionState) -> OidcResponse<OidcDiscoveryResponse> {
    let backend_base_url = state.tenant.base_url.clone();
    let control_center_url = user_utils::get_base_url(&state);

    Ok(ApplicationResponse::Json(OidcDiscoveryResponse::new(
        backend_base_url,
        control_center_url.into(),
    )))
}

static CACHED_JWKS: OnceCell<JwksResponse> = OnceCell::new();
/// Build JWKS response with public keys (all keys for token validation)
pub async fn get_jwks(state: SessionState) -> OidcResponse<&'static JwksResponse> {
    CACHED_JWKS
        .get_or_try_init(|| {
            let oidc_keys = state.conf.oidc.get_inner().get_all_keys();
            let mut keys = Vec::with_capacity(oidc_keys.len());

            for key_config in oidc_keys {
                let (n, e) = common_utils::crypto::extract_rsa_public_key_components(
                    &key_config.private_key,
                )
                .change_context(OidcErrors::ServerError)
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

            Ok(JwksResponse { keys })
        })
        .map(ApplicationResponse::Json)
}

fn validate_authorize_params(payload: &OidcAuthorizeQuery, state: &SessionState) -> OidcResult<()> {
    if !payload.scope.contains(&Scope::Openid) {
        return Err(report!(OidcErrors::InvalidScope));
    }
    let client = state
        .conf
        .oidc
        .get_inner()
        .get_client(&payload.client_id)
        .ok_or_else(|| report!(OidcErrors::UnauthorizedClient))?;

    if !oidc_utils::validate_redirect_uri_match(&client.redirect_uri, &payload.redirect_uri) {
        return Err(report!(OidcErrors::InvalidRequest));
    }

    Ok(())
}

async fn generate_and_store_authorization_code(
    state: &SessionState,
    user_id: &str,
    payload: &OidcAuthorizeQuery,
) -> OidcResult<String> {
    let user_from_db = state
        .global_store
        .find_user_by_id(user_id)
        .await
        .change_context(OidcErrors::ServerError)
        .attach_printable("Failed to fetch user from database")?;

    let auth_code =
        common_utils::crypto::generate_cryptographically_secure_random_string(AUTH_CODE_LENGTH);

    let auth_code_data = AuthCodeData {
        sub: user_id.to_string(),
        client_id: payload.client_id.clone(),
        redirect_uri: payload.redirect_uri.clone(),
        scope: payload.scope.clone(),
        nonce: payload.nonce.clone(),
        email: user_from_db.email,
        is_verified: user_from_db.is_verified,
    };

    oidc_utils::set_auth_code_in_redis(state, &auth_code, &auth_code_data)
        .await
        .attach_printable("Failed to store authorization code")?;

    Ok(auth_code)
}

pub async fn process_authorize_request(
    state: SessionState,
    payload: OidcAuthorizeQuery,
    user: Option<UserFromToken>,
) -> OidcResponse<()> {
    // Validate all parameters
    validate_authorize_params(&payload, &state)?;
    // TODO: Handle users who are not already logged in
    let user = user.ok_or_else(|| report!(OidcErrors::AccessDenied))?;

    let auth_code = generate_and_store_authorization_code(&state, &user.user_id, &payload).await?;

    let redirect_url =
        oidc_utils::build_oidc_redirect_url(&payload.redirect_uri, &auth_code, &payload.state)
            .attach_printable("Failed to build redirect URL")?;

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
) -> OidcResult<()> {
    if redirect_uri.trim().is_empty() {
        return Err(report!(OidcErrors::InvalidTokenRequest));
    }

    let registered_client = state
        .conf
        .oidc
        .get_inner()
        .get_client(client_id)
        .ok_or_else(|| report!(OidcErrors::InvalidClient))?;

    if !oidc_utils::validate_redirect_uri_match(&registered_client.redirect_uri, redirect_uri) {
        return Err(report!(OidcErrors::InvalidTokenRequest));
    }

    Ok(())
}

async fn validate_and_consume_authorization_code(
    state: &SessionState,
    code: &str,
    client_id: &str,
    redirect_uri: &str,
) -> OidcResult<AuthCodeData> {
    let auth_code_data = oidc_utils::get_auth_code_from_redis(state, code).await?;

    if let Err(err) = oidc_utils::delete_auth_code_from_redis(state, code).await {
        logger::warn!(
            error = ?err,
            "Failed to delete authorization code from Redis after consumption"
        );
    }

    if !oidc_utils::validate_client_id_match(&auth_code_data.client_id, client_id) {
        return Err(report!(OidcErrors::InvalidGrant));
    }

    if !oidc_utils::validate_redirect_uri_match(&auth_code_data.redirect_uri, redirect_uri) {
        return Err(report!(OidcErrors::InvalidGrant));
    }

    Ok(auth_code_data)
}

async fn generate_id_token(
    state: &SessionState,
    auth_code_data: &AuthCodeData,
) -> OidcResult<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .change_context(OidcErrors::ServerError)?
        .as_secs();
    let exp_duration = Duration::from_secs(ID_TOKEN_TTL_IN_SECS);
    let exp = jwt::generate_exp(exp_duration)
        .change_context(OidcErrors::ServerError)?
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

    oidc_utils::sign_oidc_token(state, claims, "ID token")
        .await
        .attach_printable("Failed to sign ID token")
}

async fn generate_access_token(
    state: &SessionState,
    auth_code_data: &AuthCodeData,
) -> OidcResult<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .change_context(OidcErrors::ServerError)?
        .as_secs();
    let exp_duration = Duration::from_secs(ACCESS_TOKEN_TTL_IN_SECS);
    let exp = jwt::generate_exp(exp_duration)
        .change_context(OidcErrors::ServerError)?
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

    oidc_utils::sign_oidc_token(state, claims, "access token")
        .await
        .attach_printable("Failed to sign access token")
}

pub async fn process_token_request(
    state: SessionState,
    payload: OidcTokenRequest,
    client_id: String,
) -> OidcResponse<OidcTokenResponse> {
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
