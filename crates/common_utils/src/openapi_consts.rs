//! This file contains the examples to be used for openapi

/// Creating the payment with minimal fields
pub const PAYMENTS_CREATE: &str = r#"{
    "amount": 6540,
    "currency": "USD",
}"#;

/// Creating a manual capture payment
pub const PAYMENTS_CREATE_WITH_MANUAL_CAPTURE: &str = r#"{
    "amount": 6540,
    "currency": "USD",
    "capture_method":"manual"
}"#;
