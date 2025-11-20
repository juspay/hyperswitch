use api_models::oidc::{Jwk, JwksResponse, OidcDiscoveryResponse, SigningAlgorithm};
use error_stack::ResultExt;
use masking::PeekInterface;

use crate::{
    core::errors::{ApiErrorResponse, RouterResponse},
    routes::app::SessionState,
    services::api::ApplicationResponse,
    utils::user::get_base_url,
};

/// Build OIDC discovery document
pub async fn get_discovery_document(state: SessionState) -> RouterResponse<OidcDiscoveryResponse> {
    let backend_base_url = state.tenant.base_url.clone();
    let frontend_base_url = get_base_url(&state).to_string();

    Ok(ApplicationResponse::Json(OidcDiscoveryResponse::new(
        backend_base_url,
        frontend_base_url,
    )))
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
            alg: SigningAlgorithm::Rs256.to_string(),
            n,
            e,
        };

        jwks.push(jwk);
    }

    let jwks_response = JwksResponse { keys: jwks };

    Ok(ApplicationResponse::Json(jwks_response))
}
