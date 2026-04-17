//! Redis command and argument constants to avoid raw string literals

/// Redis commands used in redis::cmd() and pipe.cmd() calls
pub const REDIS_CMD_SET: &str = "SET";
pub const REDIS_CMD_GET: &str = "GET";
pub const REDIS_CMD_HSCAN: &str = "HSCAN";

/// Redis arguments used in .arg() calls
pub const REDIS_ARG_EX: &str = "EX";
pub const REDIS_ARG_NX: &str = "NX";
pub const REDIS_ARG_MATCH: &str = "MATCH";
pub const REDIS_ARG_COUNT: &str = "COUNT";

/// Maximum delay between PubSub reconnection attempts
pub const PUBSUB_MAX_RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(30);
