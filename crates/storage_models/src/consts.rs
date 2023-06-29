/// API client request timeout (in seconds)
pub const REQUEST_TIME_OUT: u64 = 30;

///Payment intent fulfillment default timeout (in seconds)
pub const DEFAULT_FULFILLMENT_TIME: i64 = 15 * 60;

// General purpose base64 engines
pub(crate) const BASE64_ENGINE: base64::engine::GeneralPurpose =
    base64::engine::general_purpose::STANDARD;

pub(crate) const PUB_SUB_CHANNEL: &str = "hyperswitch_invalidate";
