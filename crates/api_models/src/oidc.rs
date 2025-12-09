use common_utils::{events::ApiEventMetric, pii};
use masking::Secret;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use utoipa::ToSchema;

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
#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseType {
    Code,
}

/// OIDC Response Mode
#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseMode {
    Query,
}

/// OIDC Subject Type
#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubjectType {
    Public,
}

/// OIDC Signing Algorithm
#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum SigningAlgorithm {
    #[serde(rename = "RS256")]
    Rs256,
}

/// JWK Key Type
#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum KeyType {
    #[serde(rename = "RSA")]
    Rsa,
}

/// JWK Key Use
#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyUse {
    Sig,
}

/// OIDC Grant Type
#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantType {
    AuthorizationCode,
}

/// OIDC Scope
#[derive(
    Copy, Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Scope {
    Openid,
    Email,
}

/// OIDC Token Endpoint Authentication Method
#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenAuthMethod {
    ClientSecretBasic,
}

/// OIDC Claim
#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Claim {
    Aud,
    Email,
    EmailVerified,
    Exp,
    Iat,
    Iss,
    Sub,
}

/// OIDC Authorization Error as per RFC 6749
#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum OidcAuthorizationError {
    InvalidRequest,
    UnauthorizedClient,
    AccessDenied,
    UnsupportedResponseType,
    InvalidScope,
    ServerError,
    TemporarilyUnavailable,
}

/// OIDC Token Error as per RFC 6749
#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum OidcTokenError {
    InvalidRequest,
    InvalidClient,
    InvalidGrant,
    UnauthorizedClient,
    UnsupportedGrantType,
    InvalidScope,
}

/// OpenID Connect Discovery Response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OidcDiscoveryResponse {
    /// The issuer identifier for the OpenID Provider
    #[schema(example = "https://sandbox.hyperswitch.io")]
    pub issuer: String,

    /// URL of the authorization endpoint
    #[schema(example = "https://app.hyperswitch.io/oauth2/authorize")]
    pub authorization_endpoint: String,

    /// URL of the token endpoint
    #[schema(example = "https://sandbox.hyperswitch.io/oauth2/token")]
    pub token_endpoint: String,

    /// URL of the JSON Web Key Set document
    #[schema(example = "https://sandbox.hyperswitch.io/oauth2/jwks")]
    pub jwks_uri: String,

    /// List of OAuth 2.0 response_type values supported
    pub response_types_supported: Vec<ResponseType>,

    /// List of OAuth 2.0 response_mode values supported
    pub response_modes_supported: Vec<ResponseMode>,

    /// List of Subject Identifier types supported
    pub subject_types_supported: Vec<SubjectType>,

    /// List of JWS signing algorithms supported for ID Tokens
    pub id_token_signing_alg_values_supported: Vec<SigningAlgorithm>,

    /// List of OAuth 2.0 grant type values supported
    pub grant_types_supported: Vec<GrantType>,

    /// List of OAuth 2.0 scope values supported
    pub scopes_supported: Vec<Scope>,

    /// List of Client Authentication methods supported by the token endpoint
    pub token_endpoint_auth_methods_supported: Vec<TokenAuthMethod>,

    /// List of Claim Names supported
    pub claims_supported: Vec<Claim>,

    /// Whether the claims parameter is supported
    #[serde(default)]
    #[schema(example = false)]
    pub claims_parameter_supported: bool,

    /// Whether the request parameter is supported
    #[serde(default)]
    #[schema(example = false)]
    pub request_parameter_supported: bool,

    /// Whether the request_uri parameter is supported
    #[serde(default)]
    #[schema(example = false)]
    pub request_uri_parameter_supported: bool,
}

impl OidcDiscoveryResponse {
    pub fn new(issuer: String, control_center_url: String) -> Self {
        let authorization_endpoint = format!("{}/oauth2/authorize", control_center_url);
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
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JwksResponse {
    /// Array of JSON Web Keys
    pub keys: Vec<Jwk>,
}

/// JSON Web Key (JWK) for RSA public key
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Jwk {
    /// Key type
    pub kty: KeyType,

    /// Key ID
    #[schema(example = "key-1")]
    pub kid: String,

    /// Public key use
    #[serde(rename = "use")]
    pub key_use: KeyUse,

    /// Algorithm
    pub alg: SigningAlgorithm,

    /// RSA public key modulus
    pub n: String,

    /// RSA public key exponent
    #[schema(example = "AQAB")]
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

/// OIDC Authorization Request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OidcAuthorizeQuery {
    /// OAuth 2.0 Response Type value
    pub response_type: ResponseType,

    /// OAuth 2.0 Client Identifier
    #[schema(example = "client_abc123")]
    pub client_id: String,

    /// Redirection URI to which the response will be sent
    #[schema(example = "https://example.com/callback")]
    pub redirect_uri: String,

    /// OpenID Connect scope values
    #[serde(deserialize_with = "deserialize_scope_vec")]
    pub scope: Vec<Scope>,

    /// Opaque value used to maintain state between request and callback
    #[schema(example = "state_xyz789")]
    pub state: String,

    /// String value used to associate a Client session with an ID Token
    #[schema(example = "nonce_abc123")]
    pub nonce: Option<String>,
}

/// Authorization Code Data stored in Redis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthCodeData {
    pub sub: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Vec<Scope>,
    pub nonce: Option<String>,
    pub email: pii::Email,
}

/// OIDC Token Request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OidcTokenRequest {
    /// OAuth 2.0 Grant Type value
    pub grant_type: GrantType,

    /// Authorization code received from the authorization server
    #[schema(example = "auth_code_xyz789")]
    pub code: String,

    /// Redirection URI that was used in the authorization request
    #[schema(example = "https://example.com/callback")]
    pub redirect_uri: String,

    /// OAuth 2.0 Client Identifier
    #[schema(example = "client_abc123")]
    pub client_id: String,
}

/// OIDC Token Response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OidcTokenResponse {
    /// ID Token value associated with the authenticated session
    #[schema(value_type = String)]
    pub id_token: Secret<String>,

    /// OAuth 2.0 Token Type value
    #[schema(example = "Bearer")]
    pub token_type: String,

    /// Expiration time of the ID Token in seconds since the response was generated
    #[schema(example = 3600)]
    pub expires_in: u64,
}

impl ApiEventMetric for OidcDiscoveryResponse {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Oidc)
    }
}

impl ApiEventMetric for JwksResponse {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Oidc)
    }
}

impl ApiEventMetric for OidcAuthorizeQuery {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Oidc)
    }
}

impl ApiEventMetric for AuthCodeData {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Oidc)
    }
}

impl ApiEventMetric for OidcTokenRequest {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Oidc)
    }
}

impl ApiEventMetric for OidcTokenResponse {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Oidc)
    }
}
