//! Constants that are used in the domain level.

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

/// Default payment intent statuses that trigger a webhook
pub const DEFAULT_PAYMENT_WEBHOOK_TRIGGER_STATUSES: &[common_enums::IntentStatus] = &[
    common_enums::IntentStatus::Succeeded,
    common_enums::IntentStatus::Failed,
    common_enums::IntentStatus::PartiallyCaptured,
    common_enums::IntentStatus::RequiresMerchantAction,
];

/// Default refund statuses that trigger a webhook
pub const DEFAULT_REFUND_WEBHOOK_TRIGGER_STATUSES: &[common_enums::RefundStatus] = &[
    common_enums::RefundStatus::Success,
    common_enums::RefundStatus::Failure,
    common_enums::RefundStatus::TransactionFailure,
];

/// Default payout statuses that trigger a webhook
pub const DEFAULT_PAYOUT_WEBHOOK_TRIGGER_STATUSES: &[common_enums::PayoutStatus] = &[
    common_enums::PayoutStatus::Success,
    common_enums::PayoutStatus::Failed,
    common_enums::PayoutStatus::Initiated,
    common_enums::PayoutStatus::Pending,
];
