use common_utils::{events::ApiEventMetric, pii};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

const RESPONSE_TYPES_SUPPORTED: &[ResponseType] = &[ResponseType::Code];
const RESPONSE_MODES_SUPPORTED: &[ResponseMode] = &[ResponseMode::Query];
const SUBJECT_TYPES_SUPPORTED: &[SubjectType] = &[SubjectType::Public];
const ID_TOKEN_SIGNING_ALGS_SUPPORTED: &[SigningAlgorithm] = &[SigningAlgorithm::Rs256];
const GRANT_TYPES_SUPPORTED: &[GrantType] = &[GrantType::AuthorizationCode];
const SCOPES_SUPPORTED: &[Scope] = &[Scope::Openid, Scope::Email];
const TOKEN_ENDPOINT_AUTH_METHODS_SUPPORTED: &[TokenAuthMethod] =
    &[TokenAuthMethod::ClientSecretBasic];
const CLAIMS_SUPPORTED: &[Claim] = &[
    Claim::Aud,
    Claim::Email,
    Claim::EmailVerified,
    Claim::Exp,
    Claim::Iat,
    Claim::Iss,
    Claim::Sub,
];

/// OIDC Response Type
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ResponseType {
    Code,
}

/// OIDC Response Mode
#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize, strum::Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ResponseMode {
    Query,
}

/// OIDC Subject Type
#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize, strum::Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SubjectType {
    Public,
}

/// OIDC Signing Algorithm
#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize, strum::Display,
)]
pub enum SigningAlgorithm {
    #[serde(rename = "RS256")]
    #[strum(serialize = "RS256")]
    Rs256,
}

/// OIDC Grant Type
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum GrantType {
    AuthorizationCode,
}

/// OIDC Scope
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Scope {
    Openid,
    Email,
}

/// OIDC Token Endpoint Authentication Method
#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize, strum::Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum TokenAuthMethod {
    ClientSecretBasic,
}

/// OIDC Claim
#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize, strum::Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Claim {
    Aud,
    Email,
    EmailVerified,
    Exp,
    Iat,
    Iss,
    Sub,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcDiscoveryResponse {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub jwks_uri: String,
    pub response_types_supported: Vec<ResponseType>,
    pub response_modes_supported: Vec<ResponseMode>,
    pub subject_types_supported: Vec<SubjectType>,
    pub id_token_signing_alg_values_supported: Vec<SigningAlgorithm>,
    pub grant_types_supported: Vec<GrantType>,
    pub scopes_supported: Vec<Scope>,
    pub token_endpoint_auth_methods_supported: Vec<TokenAuthMethod>,
    pub claims_supported: Vec<Claim>,
    #[serde(default)]
    pub claims_parameter_supported: bool,
    #[serde(default)]
    pub request_parameter_supported: bool,
    #[serde(default)]
    pub request_uri_parameter_supported: bool,
}

impl OidcDiscoveryResponse {
    pub fn new(issuer: String, frontend_base_url: String) -> Self {
        let authorization_endpoint = format!("{}/oauth2/authorize", frontend_base_url);
        let token_endpoint = format!("{}/oauth2/token", issuer);
        let jwks_uri = format!("{}/oauth2/jwks", issuer);

        Self {
            issuer,
            authorization_endpoint,
            token_endpoint,
            jwks_uri,
            response_types_supported: RESPONSE_TYPES_SUPPORTED.to_vec(),
            response_modes_supported: RESPONSE_MODES_SUPPORTED.to_vec(),
            subject_types_supported: SUBJECT_TYPES_SUPPORTED.to_vec(),
            id_token_signing_alg_values_supported: ID_TOKEN_SIGNING_ALGS_SUPPORTED.to_vec(),
            grant_types_supported: GRANT_TYPES_SUPPORTED.to_vec(),
            scopes_supported: SCOPES_SUPPORTED.to_vec(),
            token_endpoint_auth_methods_supported: TOKEN_ENDPOINT_AUTH_METHODS_SUPPORTED.to_vec(),
            claims_supported: CLAIMS_SUPPORTED.to_vec(),
            claims_parameter_supported: false,
            request_parameter_supported: false,
            request_uri_parameter_supported: false,
        }
    }
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

/// Custom deserializer for space-separated scope strings
fn deserialize_scope_vec<'de, D>(deserializer: D) -> Result<Vec<Scope>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let s = String::deserialize(deserializer)?;
    s.split_whitespace()
        .map(|scope_str| {
            Scope::from_str(scope_str)
                .map_err(|_| D::Error::custom(format!("Invalid scope: '{}'", scope_str)))
        })
        .collect()
}

/// OIDC Authorization Request body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcAuthorizeRequest {
    pub response_type: ResponseType,
    pub client_id: String,
    pub redirect_uri: String,
    #[serde(deserialize_with = "deserialize_scope_vec")]
    pub scope: Vec<Scope>,
    pub state: String,
    pub nonce: String,
}

/// Authorization Code Data stored in Redis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthCodeData {
    pub sub: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Vec<Scope>,
    pub nonce: String,
    pub email: pii::Email,
}

/// OIDC Token Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcTokenRequest {
    pub grant_type: GrantType,
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
impl ApiEventMetric for OidcAuthorizeRequest {}
impl ApiEventMetric for AuthCodeData {}
impl ApiEventMetric for OidcTokenRequest {}
impl ApiEventMetric for OidcTokenResponse {}
