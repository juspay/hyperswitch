// Application-layer defaults for Superposition-backed deployment configs.

use super::dimension_config::{RefundConfig, EphemeralKeyConfig};

/// Default for installment payments are supported for a connector+currency pair.
pub const INSTALLMENT_CONFIG_SUPPORTED: bool = false;

/// Default refund policy: 10 attempts allowed, payments up to 365 days old eligible.
pub fn refund() -> RefundConfig {
    RefundConfig {
        max_attempts: 10,
        max_age: 365,
    }
}

/// Default eph_key validity in hours: 1 hour is only allowed
pub fn eph_key_validity() -> EphemeralKeyConfig {
    EphemeralKeyConfig {
        validity: 1,
    }
}
