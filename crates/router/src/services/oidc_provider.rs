use api_models::oidc::{
    Jwk, JwksResponse, KeyType, KeyUse, OidcDiscoveryResponse, SigningAlgorithm,
};
use error_stack::ResultExt;
use once_cell::sync::OnceCell;

use crate::{
    core::errors::{ApiErrorResponse, RouterResponse},
    routes::app::SessionState,
    services::api::ApplicationResponse,
    utils::user::get_base_url,
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
        let oidc_keys = state.conf.oidc.get_inner().get_all_keys();
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
