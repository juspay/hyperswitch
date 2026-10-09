//! Constants that are used in the domain level.

/// Base value for converting percentage to decimal (e.g., 12% → 0.12)
pub const PERCENTAGE_BASE: f64 = 100.0;

/// API version
#[cfg(feature = "v1")]
pub const API_VERSION: common_enums::ApiVersion = common_enums::ApiVersion::V1;

/// API version
#[cfg(feature = "v2")]
pub const API_VERSION: common_enums::ApiVersion = common_enums::ApiVersion::V2;

/// Maximum Dispute Polling Interval In Hours
pub const MAX_DISPUTE_POLLING_INTERVAL_IN_HOURS: i32 = 24;

///Default Dispute Polling Interval In Hours
pub const DEFAULT_DISPUTE_POLLING_INTERVAL_IN_HOURS: i32 = 24;

/// Customer List Lower Limit
pub const CUSTOMER_LIST_LOWER_LIMIT: u16 = 1;

/// Customer List Upper Limit
pub const CUSTOMER_LIST_UPPER_LIMIT: u16 = 100;

/// Customer List Default Limit
pub const CUSTOMER_LIST_DEFAULT_LIMIT: u16 = 20;
