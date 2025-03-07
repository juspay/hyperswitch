/// Header Constants
pub(crate) mod headers {
    pub(crate) const ACCEPT: &str = "Accept";
    pub(crate) const API_KEY: &str = "API-KEY";
    pub(crate) const APIKEY: &str = "apikey";
    pub(crate) const API_TOKEN: &str = "Api-Token";
    pub(crate) const AUTHORIZATION: &str = "Authorization";
    pub(crate) const CONTENT_TYPE: &str = "Content-Type";
    pub(crate) const DATE: &str = "Date";
    pub(crate) const IDEMPOTENCY_KEY: &str = "Idempotency-Key";
    pub(crate) const MESSAGE_SIGNATURE: &str = "Message-Signature";
    pub(crate) const MERCHANT_ID: &str = "Merchant-ID";
    pub(crate) const REQUEST_ID: &str = "request-id";
    pub(crate) const NONCE: &str = "nonce";
    pub(crate) const TIMESTAMP: &str = "Timestamp";
    pub(crate) const TOKEN: &str = "token";
    pub(crate) const X_ACCEPT_VERSION: &str = "X-Accept-Version";
    pub(crate) const X_CC_API_KEY: &str = "X-CC-Api-Key";
    pub(crate) const X_CC_VERSION: &str = "X-CC-Version";
    pub(crate) const X_DATE: &str = "X-Date";
    pub(crate) const X_LOGIN: &str = "X-Login";
    pub(crate) const X_NN_ACCESS_KEY: &str = "X-NN-Access-Key";
    pub(crate) const X_TRANS_KEY: &str = "X-Trans-Key";
    pub(crate) const X_RANDOM_VALUE: &str = "X-RandomValue";
    pub(crate) const X_REQUEST_DATE: &str = "X-RequestDate";
    pub(crate) const X_VERSION: &str = "X-Version";
    pub(crate) const X_API_KEY: &str = "X-Api-Key";
    pub(crate) const CORRELATION_ID: &str = "Correlation-Id";
    pub(crate) const WP_API_VERSION: &str = "WP-Api-Version";
    pub(crate) const SOURCE: &str = "Source";
    pub(crate) const USER_AGENT: &str = "User-Agent";
    pub(crate) const KEY: &str = "key";
    pub(crate) const X_SIGNATURE: &str = "X-Signature";
}

/// Unsupported response type error message
pub const UNSUPPORTED_ERROR_MESSAGE: &str = "Unsupported response type";

/// Error message for Authentication Error from the connector
pub const CONNECTOR_UNAUTHORIZED_ERROR: &str = "Authentication Error from the connector";

/// Error message when Refund request has been voided.
pub const REFUND_VOIDED: &str = "Refund request has been voided.";

pub const LOW_BALANCE_ERROR_MESSAGE: &str = "Insufficient balance in the payment method";

pub const DUIT_NOW_BRAND_COLOR: &str = "#ED2E67";

pub const DUIT_NOW_BRAND_TEXT: &str = "MALAYSIA NATIONAL QR";

pub(crate) const CANNOT_CONTINUE_AUTH: &str =
    "Cannot continue with Authorization due to failed Liability Shift.";

#[cfg(feature = "payouts")]
pub(crate) const DEFAULT_NOTIFICATION_SCRIPT_LANGUAGE: &str = "en-US";
