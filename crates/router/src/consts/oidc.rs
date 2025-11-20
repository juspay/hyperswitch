/// Redis key prefix for OIDC authorization codes
pub const REDIS_AUTH_CODE_PREFIX: &str = "OIDC_AUTH_CODE_";

/// TTL for authorization codes in Redis (in seconds)
pub const AUTH_CODE_TTL: i64 = 5 * 60; // 300 seconds (5 minutes)

/// TTL for ID tokens (in seconds)
pub const ID_TOKEN_TTL: u64 = 60 * 60; // 3600 seconds (1 hour)

/// Length of generated authorization codes
pub const AUTH_CODE_LENGTH: usize = 32;
