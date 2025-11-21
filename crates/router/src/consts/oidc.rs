pub const REDIS_AUTH_CODE_PREFIX: &str = "OIDC_AUTH_CODE_";
pub const AUTH_CODE_TTL: i64 = 5 * 60; // 5 minutes
pub const ID_TOKEN_TTL: u64 = 60 * 60; // 1 hour
pub const AUTH_CODE_LENGTH: usize = 32;
