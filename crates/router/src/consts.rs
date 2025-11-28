pub mod opensearch;
#[cfg(feature = "olap")]
pub mod user;
pub mod user_role;
use std::{collections::HashSet, str::FromStr, sync};

use api_models::enums::Country;
use common_utils::{consts, id_type};
pub use hyperswitch_domain_models::consts::{
    CONNECTOR_MANDATE_REQUEST_REFERENCE_ID_LENGTH, ROUTING_ENABLED_PAYMENT_METHODS,
    ROUTING_ENABLED_PAYMENT_METHOD_TYPES,
};
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

// General purpose base64 engines

pub(crate) const BASE64_ENGINE: base64::engine::GeneralPurpose = consts::BASE64_ENGINE;

pub(crate) const API_KEY_LENGTH: usize = 64;

// OID (Object Identifier) for the merchant ID field extension.
pub(crate) const MERCHANT_ID_FIELD_EXTENSION_ID: &str = "1.2.840.113635.100.6.32";

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

pub const CARD_IP_BLOCKING_CACHE_KEY_PREFIX: &str = "CARD_IP_BLOCKING";

pub const GUEST_USER_CARD_BLOCKING_CACHE_KEY_PREFIX: &str = "GUEST_USER_CARD_BLOCKING";

pub const CUSTOMER_ID_BLOCKING_PREFIX: &str = "CUSTOMER_ID_BLOCKING";

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

// 15 minutes = 900 seconds
pub const AUTHENTICATION_ELIGIBILITY_CHECK_DATA_TTL: i64 = 900;

// Prefix key for storing authentication eligibility check data in redis
pub const AUTHENTICATION_ELIGIBILITY_CHECK_DATA_KEY: &str = "AUTH_ELIGIBILITY_CHECK_DATA_";

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

/// Default allowed domains for payment links
pub const DEFAULT_ALLOWED_DOMAINS: Option<HashSet<String>> = None;

/// Default hide card nickname field
pub const DEFAULT_HIDE_CARD_NICKNAME_FIELD: bool = false;

/// Show card form by default for payment links
pub const DEFAULT_SHOW_CARD_FORM: bool = true;

/// Default bool for Display sdk only
pub const DEFAULT_DISPLAY_SDK_ONLY: bool = false;

/// Default bool to enable saved payment method
pub const DEFAULT_ENABLE_SAVED_PAYMENT_METHOD: bool = false;

/// [PaymentLink] Default bool for enabling button only when form is ready
pub const DEFAULT_ENABLE_BUTTON_ONLY_ON_FORM_READY: bool = false;

/// Default Merchant Logo Link
pub const DEFAULT_MERCHANT_LOGO: &str =
    "https://live.hyperswitch.io/payment-link-assets/Merchant_placeholder.png";

/// Default Payment Link Background color
pub const DEFAULT_BACKGROUND_COLOR: &str = "#212E46";

/// Default product Img Link
pub const DEFAULT_PRODUCT_IMG: &str =
    "https://live.hyperswitch.io/payment-link-assets/cart_placeholder.png";

/// Default SDK Layout
pub const DEFAULT_SDK_LAYOUT: &str = "tabs";

/// Vault Add request url
#[cfg(feature = "v2")]
pub const ADD_VAULT_REQUEST_URL: &str = "/api/v2/vault/add";

/// Vault Get Fingerprint request url
#[cfg(feature = "v2")]
pub const VAULT_FINGERPRINT_REQUEST_URL: &str = "/api/v2/vault/fingerprint";

/// Vault Retrieve request url
#[cfg(feature = "v2")]
pub const VAULT_RETRIEVE_REQUEST_URL: &str = "/api/v2/vault/retrieve";

/// Vault Delete request url
#[cfg(feature = "v2")]
pub const VAULT_DELETE_REQUEST_URL: &str = "/api/v2/vault/delete";

/// Vault Header content type
#[cfg(feature = "v2")]
pub const VAULT_HEADER_CONTENT_TYPE: &str = "application/json";

/// Vault Add flow type
#[cfg(feature = "v2")]
pub const VAULT_ADD_FLOW_TYPE: &str = "add_to_vault";

/// Vault Retrieve flow type
#[cfg(feature = "v2")]
pub const VAULT_RETRIEVE_FLOW_TYPE: &str = "retrieve_from_vault";

/// Vault Delete flow type
#[cfg(feature = "v2")]
pub const VAULT_DELETE_FLOW_TYPE: &str = "delete_from_vault";

/// Vault Fingerprint fetch flow type
#[cfg(feature = "v2")]
pub const VAULT_GET_FINGERPRINT_FLOW_TYPE: &str = "get_fingerprint_vault";

/// Max volume split for Dynamic routing
pub const DYNAMIC_ROUTING_MAX_VOLUME: u8 = 100;

/// Click To Pay
pub const CLICK_TO_PAY: &str = "click_to_pay";

/// Merchant eligible for authentication service config
pub const AUTHENTICATION_SERVICE_ELIGIBLE_CONFIG: &str =
    "merchants_eligible_for_authentication_service";

/// Refund flow identifier used for performing GSM operations
pub const REFUND_FLOW_STR: &str = "refund_flow";

/// Minimum IBAN length (country-dependent), as per ISO 13616 standard
pub const IBAN_MIN_LENGTH: usize = 15;

/// Maximum IBAN length defined by the ISO 13616 standard (standard max)
pub const IBAN_MAX_LENGTH: usize = 34;

/// Minimum UK BACS account number length in digits
pub const BACS_MIN_ACCOUNT_NUMBER_LENGTH: usize = 6;

/// Maximum UK BACS account number length in digits
pub const BACS_MAX_ACCOUNT_NUMBER_LENGTH: usize = 8;

/// Fixed length of UK BACS sort code in digits (always 6)
pub const BACS_SORT_CODE_LENGTH: usize = 6;

/// Exact length of Polish Elixir system domestic account number (NRB) in digits
pub const ELIXIR_ACCOUNT_NUMBER_LENGTH: usize = 26;

/// Total length of Polish IBAN including country code and checksum (28 characters)
pub const ELIXIR_IBAN_LENGTH: usize = 28;

/// Minimum length of Swedish Bankgiro number in digits
pub const BANKGIRO_MIN_LENGTH: usize = 7;

/// Maximum length of Swedish Bankgiro number in digits
pub const BANKGIRO_MAX_LENGTH: usize = 8;

/// Minimum length of Swedish Plusgiro number in digits
pub const PLUSGIRO_MIN_LENGTH: usize = 2;

/// Maximum length of Swedish Plusgiro number in digits
pub const PLUSGIRO_MAX_LENGTH: usize = 8;

/// Default payment method session expiry
pub const DEFAULT_PAYMENT_METHOD_SESSION_EXPIRY: u32 = 15 * 60; // 15 minutes

/// Authorize flow identifier used for performing GSM operations
pub const AUTHORIZE_FLOW_STR: &str = "Authorize";

/// Protocol Version for encrypted Google Pay Token
pub(crate) const PROTOCOL: &str = "ECv2";

/// Sender ID for Google Pay Decryption
pub(crate) const SENDER_ID: &[u8] = b"Google";

/// Default value for the number of attempts to retry fetching forex rates
pub const DEFAULT_ANALYTICS_FOREX_RETRY_ATTEMPTS: u64 = 3;

/// Default payment intent id
pub const IRRELEVANT_PAYMENT_INTENT_ID: &str = "irrelevant_payment_intent_id";

/// Default payment attempt id
pub const IRRELEVANT_PAYMENT_ATTEMPT_ID: &str = "irrelevant_payment_attempt_id";

pub static PROFILE_ID_UNAVAILABLE: sync::LazyLock<id_type::ProfileId> = sync::LazyLock::new(|| {
    #[allow(clippy::expect_used)]
    id_type::ProfileId::from_str("PROFILE_ID_UNAVAIABLE")
        .expect("Failed to parse PROFILE_ID_UNAVAIABLE")
});

/// Default payment attempt id
pub const IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID: &str =
    "irrelevant_connector_request_reference_id";

// Default payment method storing TTL in redis in seconds
pub const DEFAULT_PAYMENT_METHOD_STORE_TTL: i64 = 86400; // 1 day

// List of countries that are part of the PSD2 region
pub const PSD2_COUNTRIES: [Country; 27] = [
    Country::Austria,
    Country::Belgium,
    Country::Bulgaria,
    Country::Croatia,
    Country::Cyprus,
    Country::Czechia,
    Country::Denmark,
    Country::Estonia,
    Country::Finland,
    Country::France,
    Country::Germany,
    Country::Greece,
    Country::Hungary,
    Country::Ireland,
    Country::Italy,
    Country::Latvia,
    Country::Lithuania,
    Country::Luxembourg,
    Country::Malta,
    Country::Netherlands,
    Country::Poland,
    Country::Portugal,
    Country::Romania,
    Country::Slovakia,
    Country::Slovenia,
    Country::Spain,
    Country::Sweden,
];

// Rollout percentage config prefix
pub const UCS_ROLLOUT_PERCENT_CONFIG_PREFIX: &str = "ucs_rollout_config";

// UCS feature enabled config
pub const UCS_ENABLED: &str = "ucs_enabled";

/// Header value indicating that signature-key-based authentication is used.
pub const UCS_AUTH_SIGNATURE_KEY: &str = "signature-key";

/// Header value indicating that body-key-based authentication is used.
pub const UCS_AUTH_BODY_KEY: &str = "body-key";

/// Header value indicating that header-key-based authentication is used.
pub const UCS_AUTH_HEADER_KEY: &str = "header-key";

/// Header value indicating that multi-key-based authentication is used.
pub const UCS_AUTH_MULTI_KEY: &str = "multi-auth-key";

/// Header value indicating that currency-auth-key-based authentication is used.
pub const UCS_AUTH_CURRENCY_AUTH_KEY: &str = "currency-auth-key";

/// Form field name for challenge request during creq submission
pub const CREQ_CHALLENGE_REQUEST_KEY: &str = "creq";

/// Superposition configuration keys
pub mod superposition {
    /// CVV requirement configuration key
    pub const REQUIRES_CVV: &str = "requires_cvv";
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_profile_id_unavailable_initialization() {
        // Just access the lazy static to ensure it doesn't panic during initialization
        let _profile_id = super::PROFILE_ID_UNAVAILABLE.clone();
        // If we get here without panicking, the test passes
    }
}
