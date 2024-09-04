pub mod opensearch;
#[cfg(feature = "olap")]
pub mod user;
pub mod user_role;
use std::collections::HashMap;

use common_utils::{consts, types::CardNetworkPattern};
use once_cell::sync::Lazy;
use regex::Regex;
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

pub const ROLE_CACHE_PREFIX: &str = "CR_";

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
pub const DEFAULT_UNIFIED_ERROR_CODE: &str = "UE_000";
pub const DEFAULT_UNIFIED_ERROR_MESSAGE: &str = "Something went wrong";


/// Regex for Identifying Card Network
pub const CARD_NETWORK_DATA: Lazy<HashMap<common_enums::CardNetwork, CardNetworkPattern>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert(common_enums::CardNetwork::Maestro, CardNetworkPattern {
        regex:  Regex::new(r"^(5018|5081|5044|504681|504993|5020|502260|5038|603845|603123|6304|6759|676[1-3]|6220|504834|504817|504645|504775|600206|627741)[0-9]{0,15}$").ok(), 
        allowed_card_number_length: vec![12, 13, 14, 15, 16, 17, 18, 19],
        allowed_cvc_length: vec![3, 4]
    });
    map.insert(common_enums::CardNetwork::RuPay, CardNetworkPattern {
        regex: Regex::new(r"^(508227|508[5-9]|603741|60698[5-9]|60699|607[0-8]|6079[0-7]|60798[0-4]|60800[1-9]|6080[1-9]|608[1-4]|608500|6521[5-9]|652[2-9]|6530|6531[0-4]|817290|817368|817378|353800)[0-9]*$").ok(),
        allowed_card_number_length: vec![16],
        allowed_cvc_length: vec![3],
    });
    map.insert(common_enums::CardNetwork::DinersClub, 
        CardNetworkPattern {
            regex: Regex::new(r"^(36|38|30[0-5])[0-9]{0,17}$").ok(),
            allowed_card_number_length: vec![14, 15, 16, 17, 18, 19],
            allowed_cvc_length: vec![3],
        });
    map.insert(common_enums::CardNetwork::Discover,
        CardNetworkPattern {
            regex: Regex::new(r"^(6011|65|64[4-9]|622)[0-9]*$").ok(),
            allowed_card_number_length: vec![16],
            allowed_cvc_length: vec![3],
        });
    map.insert(common_enums::CardNetwork::Mastercard,
        CardNetworkPattern {
            regex: Regex::new(r"^5[1-5][0-9]{14}$").ok(),
            allowed_card_number_length: vec![16],
            allowed_cvc_length: vec![3],
        });
    map.insert(common_enums::CardNetwork::AmericanExpress, 
        CardNetworkPattern {
            regex: Regex::new(r"^3[47][0-9]{13}$").ok(),
            allowed_card_number_length: vec![14, 15],
            allowed_cvc_length: vec![3, 4],
        });
    map.insert(common_enums::CardNetwork::Visa,
        CardNetworkPattern {
            regex:  Regex::new(r"^4[0-9]{12}(?:[0-9]{3})?$").ok(),
            allowed_card_number_length: vec![13, 14, 15, 16, 19],
            allowed_cvc_length: vec![3],
        }); 
    map.insert(common_enums::CardNetwork::JCB,
        CardNetworkPattern {
            regex:  Regex::new(r"^35[0-9]{0,14}$").ok(),
            allowed_card_number_length: vec![16],
            allowed_cvc_length: vec![3],
        });
    
    map
});

