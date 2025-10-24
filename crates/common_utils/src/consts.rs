//! Commonly used constants

/// Number of characters in a generated ID
pub const ID_LENGTH: usize = 20;

/// Characters to use for generating NanoID
pub(crate) const ALPHABETS: [char; 62] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i',
    'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B',
    'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U',
    'V', 'W', 'X', 'Y', 'Z',
];

/// TTL for token
pub const TOKEN_TTL: i64 = 900;

///an example of the frm_configs json
pub static FRM_CONFIGS_EG: &str = r#"
[{"gateway":"stripe","payment_methods":[{"payment_method":"card","payment_method_types":[{"payment_method_type":"credit","card_networks":["Visa"],"flow":"pre","action":"cancel_txn"},{"payment_method_type":"debit","card_networks":["Visa"],"flow":"pre"}]}]}]
"#;

/// Maximum limit for payments list get api
pub const PAYMENTS_LIST_MAX_LIMIT_V1: u32 = 100;
/// Maximum limit for payments list post api with filters
pub const PAYMENTS_LIST_MAX_LIMIT_V2: u32 = 50;
/// Default limit for payments list API
pub fn default_payments_list_limit() -> u32 {
    10
}

/// Average delay (in seconds) between account onboarding's API response and the changes to actually reflect at Stripe's end
pub const STRIPE_ACCOUNT_ONBOARDING_DELAY_IN_SECONDS: i64 = 15;

/// Maximum limit for payment link list get api
pub const PAYMENTS_LINK_LIST_LIMIT: u32 = 100;

/// Maximum limit for payouts list get api
pub const PAYOUTS_LIST_MAX_LIMIT_GET: u32 = 100;
/// Maximum limit for payouts list post api
pub const PAYOUTS_LIST_MAX_LIMIT_POST: u32 = 20;
/// Default limit for payouts list API
pub fn default_payouts_list_limit() -> u32 {
    10
}

/// surcharge percentage maximum precision length
pub const SURCHARGE_PERCENTAGE_PRECISION_LENGTH: u8 = 2;

/// Header Key for application overhead of a request
pub const X_HS_LATENCY: &str = "x-hs-latency";

/// Redirect url for Prophetpay
pub const PROPHETPAY_REDIRECT_URL: &str = "https://ccm-thirdparty.cps.golf/hp/tokenize/";

/// Variable which store the card token for Prophetpay
pub const PROPHETPAY_TOKEN: &str = "cctoken";

/// Payment intent default client secret expiry (in seconds)
pub const DEFAULT_SESSION_EXPIRY: i64 = 15 * 60;

/// Payment intent fulfillment time (in seconds)
pub const DEFAULT_INTENT_FULFILLMENT_TIME: i64 = 15 * 60;

/// Payment order fulfillment time (in seconds)
pub const DEFAULT_ORDER_FULFILLMENT_TIME: i64 = 15 * 60;

/// Default ttl for Extended card info  in redis (in seconds)
pub const DEFAULT_TTL_FOR_EXTENDED_CARD_INFO: u16 = 15 * 60;

/// Max ttl for Extended card info in redis (in seconds)
pub const MAX_TTL_FOR_EXTENDED_CARD_INFO: u16 = 60 * 60 * 2;

/// Default tenant to be used when multitenancy is disabled
pub const DEFAULT_TENANT: &str = "public";

/// Default tenant to be used when multitenancy is disabled
pub const TENANT_HEADER: &str = "x-tenant-id";

/// Max Length for MerchantReferenceId
pub const MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH: u8 = 64;

/// Maximum length allowed for a global id
pub const MIN_GLOBAL_ID_LENGTH: u8 = 32;

/// Minimum length required for a global id
pub const MAX_GLOBAL_ID_LENGTH: u8 = 64;

/// Minimum allowed length for MerchantReferenceId
pub const MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH: u8 = 1;

/// Length of a cell identifier in a distributed system
pub const CELL_IDENTIFIER_LENGTH: u8 = 5;

/// General purpose base64 engine
pub const BASE64_ENGINE: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;

/// URL Safe base64 engine
pub const BASE64_ENGINE_URL_SAFE: base64::engine::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE;

/// URL Safe base64 engine without padding
pub const BASE64_ENGINE_URL_SAFE_NO_PAD: base64::engine::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// Regex for matching a domain
/// Eg -
/// http://www.example.com
/// https://www.example.com
/// www.example.com
/// example.io
pub const STRICT_DOMAIN_REGEX: &str = r"^(https?://)?(([A-Za-z0-9][-A-Za-z0-9]\.)*[A-Za-z0-9][-A-Za-z0-9]*|(\d{1,3}\.){3}\d{1,3})+(:[0-9]{2,4})?$";

/// Regex for matching a wildcard domain
/// Eg -
/// *.example.com
/// *.subdomain.domain.com
/// *://example.com
/// *example.com
pub const WILDCARD_DOMAIN_REGEX: &str = r"^((\*|https?)?://)?((\*\.|[A-Za-z0-9][-A-Za-z0-9]*\.)*[A-Za-z0-9][-A-Za-z0-9]*|((\d{1,3}|\*)\.){3}(\d{1,3}|\*)|\*)(:\*|:[0-9]{2,4})?(/\*)?$";

/// Maximum allowed length for MerchantName
pub const MAX_ALLOWED_MERCHANT_NAME_LENGTH: usize = 64;

/// Default locale
pub const DEFAULT_LOCALE: &str = "en";

/// Role ID for Tenant Admin
pub const ROLE_ID_TENANT_ADMIN: &str = "tenant_admin";
/// Role ID for Org Admin
pub const ROLE_ID_ORGANIZATION_ADMIN: &str = "org_admin";
/// Role ID for Internal View Only
pub const ROLE_ID_INTERNAL_VIEW_ONLY_USER: &str = "internal_view_only";
/// Role ID for Internal Admin
pub const ROLE_ID_INTERNAL_ADMIN: &str = "internal_admin";
/// Role ID for Internal Demo
pub const ROLE_ID_INTERNAL_DEMO: &str = "internal_demo";

/// Max length allowed for Description
pub const MAX_DESCRIPTION_LENGTH: u16 = 255;

/// Max length allowed for Statement Descriptor
pub const MAX_STATEMENT_DESCRIPTOR_LENGTH: u16 = 22;
/// Payout flow identifier used for performing GSM operations
pub const PAYOUT_FLOW_STR: &str = "payout_flow";

/// length of the publishable key
pub const PUBLISHABLE_KEY_LENGTH: u16 = 39;

/// The number of bytes allocated for the hashed connector transaction ID.
/// Total number of characters equals CONNECTOR_TRANSACTION_ID_HASH_BYTES times 2.
pub const CONNECTOR_TRANSACTION_ID_HASH_BYTES: usize = 25;

/// Apple Pay validation url
pub const APPLEPAY_VALIDATION_URL: &str =
    "https://apple-pay-gateway-cert.apple.com/paymentservices/startSession";

/// Request ID
pub const X_REQUEST_ID: &str = "x-request-id";

/// Flow name
pub const X_FLOW_NAME: &str = "x-flow";

/// Connector name
pub const X_CONNECTOR_NAME: &str = "x-connector";

/// Unified Connector Service Mode
pub const X_UNIFIED_CONNECTOR_SERVICE_MODE: &str = "x-shadow-mode";

/// Chat Session ID
pub const X_CHAT_SESSION_ID: &str = "x-chat-session-id";

/// Merchant ID Header
pub const X_MERCHANT_ID: &str = "x-merchant-id";

/// Default Tenant ID for the `Global` tenant
pub const DEFAULT_GLOBAL_TENANT_ID: &str = "global";

/// Default status of Card IP Blocking
pub const DEFAULT_CARD_IP_BLOCKING_STATUS: bool = false;

/// Default Threshold for Card IP Blocking
pub const DEFAULT_CARD_IP_BLOCKING_THRESHOLD: i32 = 3;

/// Default status of Guest User Card Blocking
pub const DEFAULT_GUEST_USER_CARD_BLOCKING_STATUS: bool = false;

/// Default Threshold for Card Blocking for Guest Users
pub const DEFAULT_GUEST_USER_CARD_BLOCKING_THRESHOLD: i32 = 10;

/// Default status of Customer ID Blocking
pub const DEFAULT_CUSTOMER_ID_BLOCKING_STATUS: bool = false;

/// Default Threshold for Customer ID Blocking
pub const DEFAULT_CUSTOMER_ID_BLOCKING_THRESHOLD: i32 = 5;

/// Default Card Testing Guard Redis Expiry in seconds
pub const DEFAULT_CARD_TESTING_GUARD_EXPIRY_IN_SECS: i32 = 3600;

/// SOAP 1.1 Envelope Namespace
pub const SOAP_ENV_NAMESPACE: &str = "http://schemas.xmlsoap.org/soap/envelope/";

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
/// Length of generated tokens
pub const TOKEN_LENGTH: usize = 32;

/// The tag name used for identifying the host in metrics.
pub const METRICS_HOST_TAG_NAME: &str = "host";

/// API client request timeout (in seconds)
pub const REQUEST_TIME_OUT: u64 = 30;

/// API client request timeout for ai service (in seconds)
pub const REQUEST_TIME_OUT_FOR_AI_SERVICE: u64 = 120;

/// Default limit for list operations (can be used across different entities)
pub const DEFAULT_LIST_LIMIT: i64 = 100;

/// Default offset for list operations (can be used across different entities)
pub const DEFAULT_LIST_OFFSET: i64 = 0;
