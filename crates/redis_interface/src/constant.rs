//! Shared constants for the redis_interface crate.
//!
//! Backend-specific command string constants live in each backend's module.

/// Default reconnect max attempts, matching RedisSettings::default().reconnect_max_attempts
pub const DEFAULT_RECONNECT_MAX_ATTEMPTS: usize = 5;

/// Redis commands used in redis::cmd() and pipe.cmd() calls (redis-rs backend)
#[cfg(feature = "redis-rs")]
pub mod redis_rs_commands {
    pub const REDIS_COMMAND_SET: &str = "SET";
    pub const REDIS_COMMAND_GET: &str = "GET";
    pub const REDIS_COMMAND_HSCAN: &str = "HSCAN";
    pub const REDIS_COMMAND_SCAN: &str = "SCAN";

    /// Redis arguments used in .arg() calls
    pub const REDIS_ARG_EX: &str = "EX";
    pub const REDIS_ARG_NX: &str = "NX";
    pub const REDIS_ARG_MATCH: &str = "MATCH";
    pub const REDIS_ARG_COUNT: &str = "COUNT";
    pub const REDIS_ARG_TYPE: &str = "TYPE";
}
