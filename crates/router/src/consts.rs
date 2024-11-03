pub mod opensearch;
#[cfg(feature = "olap")]
pub mod user;
pub mod user_role;

use common_utils::consts;
pub use hyperswitch_interfaces::consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE};
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

pub const DEFAULT_LIST_API_LIMIT: u16 = 10;

// String literals
pub(crate) const UNSUPPORTED_ERROR_MESSAGE: &str = "Unsupported response type";
pub(crate) const LOW_BALANCE_ERROR_MESSAGE: &str = "Insufficient balance in the payment method";
pub(crate) const CONNECTOR_UNAUTHORIZED_ERROR: &str = "Authentication Error from the connector";
pub(crate) const REFUND_VOIDED: &str = "Refund request has been voided.";

pub(crate) const CANNOT_CONTINUE_AUTH: &str =
    "Cannot continue with Authorization due to failed Liability Shift.";
#[cfg(feature = "payouts")]
pub(crate) const DEFAULT_NOTIFICATION_SCRIPT_LANGUAGE: &str = "en-US";

// General purpose base64 engines

pub(crate) const BASE64_ENGINE: base64::engine::GeneralPurpose = consts::BASE64_ENGINE;

pub(crate) const BASE64_ENGINE_URL_SAFE: base64::engine::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE;

pub(crate) const API_KEY_LENGTH: usize = 64;

// Apple Pay validation url
pub(crate) const APPLEPAY_VALIDATION_URL: &str =
    "https://apple-pay-gateway-cert.apple.com/paymentservices/startSession";

// OID (Object Identifier) for the merchant ID field extension.
pub(crate) const MERCHANT_ID_FIELD_EXTENSION_ID: &str = "1.2.840.113635.100.6.32";

pub(crate) const METRICS_HOST_TAG_NAME: &str = "host";
pub const MAX_ROUTING_CONFIGS_PER_MERCHANT: usize = 100;
pub const ROUTING_CONFIG_ID_LENGTH: usize = 10;

pub const LOCKER_REDIS_PREFIX: &str = "LOCKER_PM_TOKEN";
pub const LOCKER_REDIS_EXPIRY_SECONDS: u32 = 60 * 15; // 15 minutes

pub const JWT_TOKEN_TIME_IN_SECS: u64 = 60 * 60 * 24 * 2; // 2 days

// This should be one day, but it is causing issue while checking token in blacklist.
// TODO: This should be fixed in future.
pub const SINGLE_PURPOSE_TOKEN_TIME_IN_SECS: u64 = 60 * 60 * 24 * 2; // 2 days

pub const JWT_TOKEN_COOKIE_NAME: &str = "login_token";

pub const USER_BLACKLIST_PREFIX: &str = "BU_";

pub const ROLE_BLACKLIST_PREFIX: &str = "BR_";

#[cfg(feature = "email")]
pub const EMAIL_TOKEN_TIME_IN_SECS: u64 = 60 * 60 * 24; // 1 day

#[cfg(feature = "email")]
pub const EMAIL_TOKEN_BLACKLIST_PREFIX: &str = "BET_";

pub const EMAIL_SUBJECT_API_KEY_EXPIRY: &str = "API Key Expiry Notice";
pub const EMAIL_SUBJECT_DASHBOARD_FEATURE_REQUEST: &str = "Dashboard Pro Feature Request by";
pub const EMAIL_SUBJECT_APPROVAL_RECON_REQUEST: &str =
    "Approval of Recon Request - Access Granted to Recon Dashboard";

pub const ROLE_INFO_CACHE_PREFIX: &str = "CR_INFO_";

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

/// Max payment intent fulfillment expiry
pub const MAX_INTENT_FULFILLMENT_EXPIRY: u32 = 1800;

/// Min payment intent fulfillment expiry
pub const MIN_INTENT_FULFILLMENT_EXPIRY: u32 = 60;

pub const LOCKER_HEALTH_CALL_PATH: &str = "/health";

pub const AUTHENTICATION_ID_PREFIX: &str = "authn";

// URL for checking the outgoing call
pub const OUTGOING_CALL_URL: &str = "https://api.stripe.com/healthcheck";

// 15 minutes = 900 seconds
pub const POLL_ID_TTL: i64 = 900;

// Default Poll Config
pub const DEFAULT_POLL_DELAY_IN_SECS: i8 = 2;
pub const DEFAULT_POLL_FREQUENCY: i8 = 5;

// Number of seconds to subtract from access token expiry
pub(crate) const REDUCE_ACCESS_TOKEN_EXPIRY_TIME: u8 = 15;
pub const CONNECTOR_CREDS_TOKEN_TTL: i64 = 900;

//max_amount allowed is 999999999 in minor units
pub const MAX_ALLOWED_AMOUNT: i64 = 999999999;

//payment attempt default unified error code and unified error message
pub const DEFAULT_UNIFIED_ERROR_CODE: &str = "UE_9000";
pub const DEFAULT_UNIFIED_ERROR_MESSAGE: &str = "Something went wrong";

// Recon's feature tag
pub const RECON_FEATURE_TAG: &str = "RECONCILIATION AND SETTLEMENT";

// Length of the unique reference ID generated for connector mandate requests
pub const CONNECTOR_MANDATE_REQUEST_REFERENCE_ID_LENGTH: usize = 18;

/// Vault Add request url
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
pub const ADD_VAULT_REQUEST_URL: &str = "/vault/add";

/// Vault Get Fingerprint request url
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
pub const VAULT_FINGERPRINT_REQUEST_URL: &str = "/fingerprint";

/// Vault Retrieve request url
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
pub const VAULT_RETRIEVE_REQUEST_URL: &str = "/vault/retrieve";

/// Vault Delete request url
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
pub const VAULT_DELETE_REQUEST_URL: &str = "/vault/delete";

/// Vault Header content type
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
pub const VAULT_HEADER_CONTENT_TYPE: &str = "application/json";

/// Vault Add flow type
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
pub const VAULT_ADD_FLOW_TYPE: &str = "add_to_vault";

/// Vault Retrieve flow type
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
pub const VAULT_RETRIEVE_FLOW_TYPE: &str = "retrieve_from_vault";

/// Vault Delete flow type
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
pub const VAULT_DELETE_FLOW_TYPE: &str = "delete_from_vault";

/// Vault Fingerprint fetch flow type
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
pub const VAULT_GET_FINGERPRINT_FLOW_TYPE: &str = "get_fingerprint_vault";

/// Worldpay's unique reference ID for a request TODO: Move to hyperswitch_connectors/constants once Worldpay is moved to connectors crate
pub const WP_CORRELATION_ID: &str = "WP-CorrelationId";
