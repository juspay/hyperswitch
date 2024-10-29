//! connector integration related const declarations

/// No error message string const
pub const NO_ERROR_MESSAGE: &str = "No error message";

/// No error code string const
pub const NO_ERROR_CODE: &str = "No error code";

/// Accepted format for request
pub const ACCEPT_HEADER: &str = "text/html,application/json";

/// User agent for request send from backend server
pub const USER_AGENT: &str = "Hyperswitch-Backend-Server";

/// Unsupported response type error message
pub const UNSUPPORTED_ERROR_MESSAGE: &str = "Unsupported response type";

/// Worldpay's unique reference ID for a request TODO: Move to hyperswitch_connectors/constants once Worldpay is moved to connectors crate
pub const WP_CORRELATION_ID: &str = "WP-CorrelationId";
