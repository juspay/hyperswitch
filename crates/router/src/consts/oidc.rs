/// Redis key prefix for OIDC authorization codes
pub const REDIS_AUTH_CODE_PREFIX: &str = "OIDC_AUTH_CODE_";

/// TTL for authorization codes in Redis (in seconds)
pub const AUTH_CODE_TTL: i64 = 5 * 60; // 300 seconds (5 minutes)

/// TTL for ID tokens (in seconds)
pub const ID_TOKEN_TTL: u64 = 60 * 60; // 3600 seconds (1 hour)

/// Length of generated authorization codes
pub const AUTH_CODE_LENGTH: usize = 32;

// OIDC Discovery Document Constants
pub const RESPONSE_TYPE_CODE: &str = "code";
pub const RESPONSE_MODE_QUERY: &str = "query";
pub const SUBJECT_TYPE_PUBLIC: &str = "public";
pub const SIGNING_ALG_RS256: &str = "RS256";
pub const GRANT_TYPE_AUTHORIZATION_CODE: &str = "authorization_code";
pub const SCOPE_OPENID: &str = "openid";
pub const SCOPE_EMAIL: &str = "email";
pub const TOKEN_AUTH_METHOD_CLIENT_SECRET_BASIC: &str = "client_secret_basic";

// OIDC Claims
pub const CLAIM_AUD: &str = "aud";
pub const CLAIM_EMAIL: &str = "email";
pub const CLAIM_EMAIL_VERIFIED: &str = "email_verified";
pub const CLAIM_EXP: &str = "exp";
pub const CLAIM_IAT: &str = "iat";
pub const CLAIM_ISS: &str = "iss";
pub const CLAIM_SUB: &str = "sub";
