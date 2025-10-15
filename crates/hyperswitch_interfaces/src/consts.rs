//! connector integration related const declarations

/// No error message string const
pub const NO_ERROR_MESSAGE: &str = "No error message";

/// No error code string const
pub const NO_ERROR_CODE: &str = "No error code";

/// Accepted format for request
pub const ACCEPT_HEADER: &str = "text/html,application/json";

/// User agent for request send from backend server
pub const USER_AGENT: &str = "Hyperswitch-Backend-Server";

/// Request timeout error code
pub const REQUEST_TIMEOUT_ERROR_CODE: &str = "TIMEOUT";

/// error message for timed out request
pub const REQUEST_TIMEOUT_ERROR_MESSAGE: &str = "Connector did not respond in specified time";

/// Header value indicating that signature-key-based authentication is used.
pub const UCS_AUTH_SIGNATURE_KEY: &str = "signature-key";

/// Header value indicating that body-key-based authentication is used.
pub const UCS_AUTH_BODY_KEY: &str = "body-key";

/// Header value indicating that header-key-based authentication is used.
pub const UCS_AUTH_HEADER_KEY: &str = "header-key";

/// Header value indicating that currency-auth-key-based authentication is used.
pub const UCS_AUTH_CURRENCY_AUTH_KEY: &str = "currency-auth-key";
