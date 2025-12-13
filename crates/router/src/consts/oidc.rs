pub const REDIS_AUTH_CODE_PREFIX: &str = "OIDC_AUTH_CODE_";
pub const AUTH_CODE_TTL_IN_SECS: i64 = 60 * 5; // 5 minutes
pub const ID_TOKEN_TTL_IN_SECS: u64 = 60 * 60; // 1 hour
pub const AUTH_CODE_LENGTH: usize = 32;
pub const TOKEN_TYPE_BEARER: &str = "Bearer";
