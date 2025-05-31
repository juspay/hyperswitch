pub mod configs;
pub mod connection;
pub mod connector;
pub mod consts;
pub mod core;
pub mod cors;
pub mod db;
pub mod env;
pub mod locale;
pub(crate) mod macros;

pub mod routes;
pub mod workflows;

#[cfg(feature = "olap")]
pub mod analytics;
pub mod analytics_validator;
pub mod events;
pub mod middleware;
pub mod services;
pub mod types;
pub mod utils;

use routes::{AppState, SessionState};

pub use self::env::logger;
pub(crate) use self::macros::*;
use crate::configs::settings;
pub use crate::core::errors;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

// Import translate fn in root
use crate::locale::{_rust_i18n_t, _rust_i18n_try_translate};

/// Header Constants
pub mod headers {
    pub const ACCEPT: &str = "Accept";
    pub const ACCEPT_LANGUAGE: &str = "Accept-Language";
    pub const KEY: &str = "key";
    pub const API_KEY: &str = "API-KEY";
    pub const APIKEY: &str = "apikey";
    pub const X_CC_API_KEY: &str = "X-CC-Api-Key";
    pub const API_TOKEN: &str = "Api-Token";
    pub const AUTHORIZATION: &str = "Authorization";
    pub const CONTENT_TYPE: &str = "Content-Type";
    pub const DATE: &str = "Date";
    pub const IDEMPOTENCY_KEY: &str = "Idempotency-Key";
    pub const NONCE: &str = "nonce";
    pub const TIMESTAMP: &str = "Timestamp";
    pub const TOKEN: &str = "token";
    pub const USER_AGENT: &str = "User-Agent";
    pub const X_API_KEY: &str = "X-API-KEY";
    pub const X_API_VERSION: &str = "X-ApiVersion";
    pub const X_FORWARDED_FOR: &str = "X-Forwarded-For";
    pub const X_MERCHANT_ID: &str = "X-Merchant-Id";
    pub const X_ORGANIZATION_ID: &str = "X-Organization-Id";
    pub const X_LOGIN: &str = "X-Login";
    pub const X_TRANS_KEY: &str = "X-Trans-Key";
    pub const X_VERSION: &str = "X-Version";
    pub const X_CC_VERSION: &str = "X-CC-Version";
    pub const X_ACCEPT_VERSION: &str = "X-Accept-Version";
    pub const X_DATE: &str = "X-Date";
    pub const X_WEBHOOK_SIGNATURE: &str = "X-Webhook-Signature-512";
    pub const X_REQUEST_ID: &str = "X-Request-Id";
    pub const X_PROFILE_ID: &str = "X-Profile-Id";
    pub const STRIPE_COMPATIBLE_WEBHOOK_SIGNATURE: &str = "Stripe-Signature";
    pub const STRIPE_COMPATIBLE_CONNECT_ACCOUNT: &str = "Stripe-Account";
    pub const X_CLIENT_VERSION: &str = "X-Client-Version";
    pub const X_CLIENT_SOURCE: &str = "X-Client-Source";
    pub const X_PAYMENT_CONFIRM_SOURCE: &str = "X-Payment-Confirm-Source";
    pub const CONTENT_LENGTH: &str = "Content-Length";
    pub const BROWSER_NAME: &str = "x-browser-name";
    pub const X_CLIENT_PLATFORM: &str = "x-client-platform";
    pub const X_MERCHANT_DOMAIN: &str = "x-merchant-domain";
    pub const X_APP_ID: &str = "x-app-id";
    pub const X_REDIRECT_URI: &str = "x-redirect-uri";
    pub const X_TENANT_ID: &str = "x-tenant-id";
    pub const X_CLIENT_SECRET: &str = "X-Client-Secret";
    pub const X_CUSTOMER_ID: &str = "X-Customer-Id";
    pub const X_CONNECTED_MERCHANT_ID: &str = "x-connected-merchant-id";
}

pub mod pii {
    //! Personal Identifiable Information protection.

    pub use common_utils::pii::Email;
    #[doc(inline)]
    pub use masking::*;
}
