pub const MAX_NAME_LENGTH: usize = 70;
pub const MAX_COMPANY_NAME_LENGTH: usize = 70;
pub const BUSINESS_EMAIL: &str = "biz@hyperswitch.io";
pub const RECOVERY_CODES_COUNT: usize = 8;
pub const RECOVERY_CODE_LENGTH: usize = 8; // This is without counting the hyphen in between
pub const TOTP_ISSUER_NAME: &str = "Hyperswitch";
/// The number of digits composing the auth code.
pub const TOTP_DIGITS: usize = 6;
/// Duration in seconds of a step.
pub const TOTP_VALIDITY_DURATION_IN_SECONDS: u64 = 30;
/// Number of totps allowed as network delay. 1 would mean one totp before current totp and one totp after are valids.
pub const TOTP_TOLERANCE: u8 = 1;
pub const MAX_PASSWORD_LENGTH: usize = 70;
pub const MIN_PASSWORD_LENGTH: usize = 8;

pub const TOTP_PREFIX: &str = "TOTP_";
pub const REDIS_RECOVERY_CODE_PREFIX: &str = "RC_";
