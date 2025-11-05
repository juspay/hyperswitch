use common_utils::events::ApiEventMetric;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcDiscoveryResponse {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub jwks_uri: String,
    pub response_types_supported: Vec<String>,
    pub response_modes_supported: Vec<String>,
    pub subject_types_supported: Vec<String>,
    pub id_token_signing_alg_values_supported: Vec<String>,
    pub grant_types_supported: Vec<String>,
    pub scopes_supported: Vec<String>,
    pub token_endpoint_auth_methods_supported: Vec<String>,
    pub claims_supported: Vec<String>,
}

/// JWKS (JSON Web Key Set) response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwksResponse {
    pub keys: Vec<Jwk>,
}

/// JSON Web Key (JWK) for RSA public key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jwk {
    pub kty: String,
    pub kid: String,
    #[serde(rename = "use")]
    pub key_use: String,
    pub alg: String,
    pub n: String,
    pub e: String,
}

/// OIDC Error Query Parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcErrorQuery {
    pub error: String,
    pub state: Option<String>,
    pub error_description: Option<String>,
}

/// OIDC Authorization Endpoint Query Parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcAuthorizeQuery {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: String,
    pub state: Option<String>,
    pub nonce: Option<String>,
}

/// Authorization Code Data stored in Redis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthCodeData {
    pub sub: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: String,
    pub nonce: Option<String>,
    pub email: String,
}

/// OIDC Token Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcTokenRequest {
    pub grant_type: String,
    pub code: String,
    pub redirect_uri: String,
    pub client_id: String,
}

/// OIDC Token Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcTokenResponse {
    pub id_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

// Event metric implementations for OIDC types
impl ApiEventMetric for OidcDiscoveryResponse {}
impl ApiEventMetric for JwksResponse {}
impl ApiEventMetric for OidcErrorQuery {}
impl ApiEventMetric for OidcAuthorizeQuery {}
impl ApiEventMetric for AuthCodeData {}
impl ApiEventMetric for OidcTokenRequest {}
impl ApiEventMetric for OidcTokenResponse {}
