//! Redis command and argument constants to avoid raw string literals

/// Redis commands used in redis::cmd() and pipe.cmd() calls
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
