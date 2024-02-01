#[cfg(feature = "olap")]
pub mod user;
pub mod user_role;

// ID generation
pub(crate) const ID_LENGTH: usize = 20;
pub(crate) const MAX_ID_LENGTH: usize = 64;
#[rustfmt::skip]
pub(crate) const ALPHABETS: [char; 62] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm',
    'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M',
    'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
];
/// API client request timeout (in seconds)
pub const REQUEST_TIME_OUT: u64 = 30;
pub const REQUEST_TIMEOUT_ERROR_CODE: &str = "TIMEOUT";
pub const REQUEST_TIMEOUT_ERROR_MESSAGE: &str = "Connector did not respond in specified time";
pub const REQUEST_TIMEOUT_PAYMENT_NOT_FOUND: &str = "Timed out ,payment not found";
pub const REQUEST_TIMEOUT_ERROR_MESSAGE_FROM_PSYNC: &str =
    "This Payment has been moved to failed as there is no response from the connector";

///Payment intent fulfillment default timeout (in seconds)
pub const DEFAULT_FULFILLMENT_TIME: i64 = 15 * 60;

/// Payment intent default client secret expiry (in seconds)
pub const DEFAULT_SESSION_EXPIRY: i64 = 15 * 60;

/// The length of a merchant fingerprint secret
pub const FINGERPRINT_SECRET_LENGTH: usize = 64;

// String literals
pub(crate) const NO_ERROR_MESSAGE: &str = "No error message";
pub(crate) const NO_ERROR_CODE: &str = "No error code";
pub(crate) const UNSUPPORTED_ERROR_MESSAGE: &str = "Unsupported response type";
pub(crate) const LOW_BALANCE_ERROR_MESSAGE: &str = "Insufficient balance in the payment method";
pub(crate) const CONNECTOR_UNAUTHORIZED_ERROR: &str = "Authentication Error from the connector";
pub(crate) const CANNOT_CONTINUE_AUTH: &str =
    "Cannot continue with Authorization due to failed Liability Shift.";

// General purpose base64 engines
pub(crate) const BASE64_ENGINE: base64::engine::GeneralPurpose =
    base64::engine::general_purpose::STANDARD;
pub(crate) const BASE64_ENGINE_URL_SAFE: base64::engine::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE;

pub(crate) const API_KEY_LENGTH: usize = 64;
pub(crate) const PUB_SUB_CHANNEL: &str = "hyperswitch_invalidate";

// Apple Pay validation url
pub(crate) const APPLEPAY_VALIDATION_URL: &str =
    "https://apple-pay-gateway-cert.apple.com/paymentservices/startSession";

// Qr Image data source starts with this string
// The base64 image data will be appended to it to image data source
pub(crate) const QR_IMAGE_DATA_SOURCE_STRING: &str = "data:image/png;base64";

// OID (Object Identifier) for the merchant ID field extension.
pub(crate) const MERCHANT_ID_FIELD_EXTENSION_ID: &str = "1.2.840.113635.100.6.32";

pub(crate) const METRICS_HOST_TAG_NAME: &str = "host";
pub const MAX_ROUTING_CONFIGS_PER_MERCHANT: usize = 100;
pub const ROUTING_CONFIG_ID_LENGTH: usize = 10;

pub const LOCKER_REDIS_PREFIX: &str = "LOCKER_PM_TOKEN";
pub const LOCKER_REDIS_EXPIRY_SECONDS: u32 = 60 * 15; // 15 minutes

pub const JWT_TOKEN_TIME_IN_SECS: u64 = 60 * 60 * 24 * 2; // 2 days

pub const USER_BLACKLIST_PREFIX: &str = "BU_";

#[cfg(feature = "email")]
pub const EMAIL_TOKEN_TIME_IN_SECS: u64 = 60 * 60 * 24; // 1 day

#[cfg(feature = "email")]
pub const EMAIL_TOKEN_BLACKLIST_PREFIX: &str = "BET_";

#[cfg(feature = "olap")]
pub const VERIFY_CONNECTOR_ID_PREFIX: &str = "conn_verify";
#[cfg(feature = "olap")]
pub const VERIFY_CONNECTOR_MERCHANT_ID: &str = "test_merchant";

#[cfg(feature = "olap")]
pub const CONNECTOR_ONBOARDING_CONFIG_PREFIX: &str = "onboarding";

/// Max payment session expiry
pub const MAX_SESSION_EXPIRY: u32 = 7890000;

/// Min payment session expiry
pub const MIN_SESSION_EXPIRY: u32 = 60;

pub const LOCKER_HEALTH_CALL_PATH: &str = "/health";

// URL for checking the outgoing call
pub const OUTGOING_CALL_URL: &str = "https://www.google.com";
