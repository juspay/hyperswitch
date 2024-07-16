use common_utils::consts::MAX_ALLOWED_MERCHANT_NAME_LENGTH;

pub const MAX_NAME_LENGTH: usize = 70;

/// The max length of company name and merchant should be same
/// because we are deriving the merchant name from company name
pub const MAX_COMPANY_NAME_LENGTH: usize = MAX_ALLOWED_MERCHANT_NAME_LENGTH;
pub const BUSINESS_EMAIL: &str = "biz@hyperswitch.io";

pub const RECOVERY_CODES_COUNT: usize = 8;
pub const RECOVERY_CODE_LENGTH: usize = 8; // This is without counting the hyphen in between

/// The number of digits composing the auth code.
pub const TOTP_DIGITS: usize = 6;
/// Duration in seconds of a step.
pub const TOTP_VALIDITY_DURATION_IN_SECONDS: u64 = 30;
/// Number of totps allowed as network delay. 1 would mean one totp before current totp and one totp after are valids.
pub const TOTP_TOLERANCE: u8 = 1;

pub const MAX_PASSWORD_LENGTH: usize = 70;
pub const MIN_PASSWORD_LENGTH: usize = 8;

pub const REDIS_TOTP_PREFIX: &str = "TOTP_";
pub const REDIS_RECOVERY_CODE_PREFIX: &str = "RC_";
pub const REDIS_TOTP_SECRET_PREFIX: &str = "TOTP_SEC_";
pub const REDIS_TOTP_SECRET_TTL_IN_SECS: i64 = 15 * 60; // 15 minutes

pub const REDIS_SSO_PREFIX: &str = "SSO_";
pub const REDIS_SSO_TTL: i64 = 5 * 60; // 5 minutes
