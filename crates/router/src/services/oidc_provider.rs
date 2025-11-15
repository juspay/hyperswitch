use api_models::oidc::{Jwk, JwksResponse, OidcDiscoveryResponse};
use error_stack::ResultExt;
use masking::PeekInterface;

use crate::{
    consts::oidc::{
        CLAIM_AUD, CLAIM_EMAIL, CLAIM_EMAIL_VERIFIED, CLAIM_EXP, CLAIM_IAT, CLAIM_ISS, CLAIM_SUB,
        GRANT_TYPE_AUTHORIZATION_CODE, RESPONSE_MODE_QUERY, RESPONSE_TYPE_CODE, SCOPE_EMAIL,
        SCOPE_OPENID, SIGNING_ALG_RS256, SUBJECT_TYPE_PUBLIC,
        TOKEN_AUTH_METHOD_CLIENT_SECRET_BASIC,
    },
    core::errors::{ApiErrorResponse, RouterResponse},
    routes::app::SessionState,
    services::api::ApplicationResponse,
    utils::user::get_base_url,
};

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
