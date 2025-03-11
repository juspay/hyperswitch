use common_enums;
use common_utils::consts::MAX_ALLOWED_MERCHANT_NAME_LENGTH;

pub const MAX_NAME_LENGTH: usize = 70;

/// The max length of company name and merchant should be same
/// because we are deriving the merchant name from company name
pub const MAX_COMPANY_NAME_LENGTH: usize = MAX_ALLOWED_MERCHANT_NAME_LENGTH;

pub const RECOVERY_CODES_COUNT: usize = 8;
pub const RECOVERY_CODE_LENGTH: usize = 8; // This is without counting the hyphen in between

/// The number of digits composing the auth code.
pub const TOTP_DIGITS: usize = 6;
/// Duration in seconds of a step.
pub const TOTP_VALIDITY_DURATION_IN_SECONDS: u64 = 30;
/// Number of totps allowed as network delay. 1 would mean one totp before current totp and one totp after are valids.
pub const TOTP_TOLERANCE: u8 = 1;
/// Number of maximum attempts user has for totp
pub const TOTP_MAX_ATTEMPTS: u8 = 4;
/// Number of maximum attempts user has for recovery code
pub const RECOVERY_CODE_MAX_ATTEMPTS: u8 = 4;
/// The default number of organizations to fetch for a tenant-level user
pub const ORG_LIST_LIMIT_FOR_TENANT: u32 = 20;

pub const MAX_PASSWORD_LENGTH: usize = 70;
pub const MIN_PASSWORD_LENGTH: usize = 8;

pub const REDIS_TOTP_PREFIX: &str = "TOTP_";
pub const REDIS_RECOVERY_CODE_PREFIX: &str = "RC_";
pub const REDIS_TOTP_SECRET_PREFIX: &str = "TOTP_SEC_";
pub const REDIS_TOTP_SECRET_TTL_IN_SECS: i64 = 15 * 60; // 15 minutes
pub const REDIS_TOTP_ATTEMPTS_PREFIX: &str = "TOTP_ATTEMPTS_";
pub const REDIS_RECOVERY_CODE_ATTEMPTS_PREFIX: &str = "RC_ATTEMPTS_";
pub const REDIS_TOTP_ATTEMPTS_TTL_IN_SECS: i64 = 5 * 60; // 5 mins
pub const REDIS_RECOVERY_CODE_ATTEMPTS_TTL_IN_SECS: i64 = 10 * 60; // 10 mins

pub const REDIS_SSO_PREFIX: &str = "SSO_";
pub const REDIS_SSO_TTL: i64 = 5 * 60; // 5 minutes

/// Email subject
pub const EMAIL_SUBJECT_SIGNUP: &str = "Welcome to the Hyperswitch community!";
pub const EMAIL_SUBJECT_INVITATION: &str = "You have been invited to join Hyperswitch Community!";
pub const EMAIL_SUBJECT_MAGIC_LINK: &str = "Unlock Hyperswitch: Use Your Magic Link to Sign In";
pub const EMAIL_SUBJECT_RESET_PASSWORD: &str = "Get back to Hyperswitch - Reset Your Password Now";
pub const EMAIL_SUBJECT_NEW_PROD_INTENT: &str = "New Prod Intent";
pub const EMAIL_SUBJECT_WELCOME_TO_COMMUNITY: &str =
    "Thank you for signing up on Hyperswitch Dashboard!";

pub const DEFAULT_PROFILE_NAME: &str = "default";
pub const DEFAULT_PRODUCT_TYPE: common_enums::MerchantProductType =
    common_enums::MerchantProductType::Orchestration;
