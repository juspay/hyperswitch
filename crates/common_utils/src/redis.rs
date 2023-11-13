//!
//! This module defines a Rust enum `RedisKey` that facilitates the creation of Redis keys for various purposes.
//!
//! # Example
//!
//! ```rust
//! let access_token_key = RedisKey::AccsesToken {
//!     merchant_id: "12345",
//!     connector_name: "example_connector",
//! };
//!
//! // Converting the Redis key to a string
//! let key_string = access_token_key.to_string();
//! println!("Redis Key: {}", key_string);
//! ```
//!
//! The `RedisKey` enum has variants representing different types of Redis keys, each with associated data:
//!
//! - `AccsesToken`: Represents a key for access tokens with merchant ID and connector name.
//! - `McdCredsId`: Represents a key for merchant credentials with merchant ID and credentials identifier.
//! - `ConnectorResponse`: Represents a key for connector responses with merchant ID, payment ID, and attempt ID.
//! - `PaymentId`: Represents a key for payment with payment ID.
//! - `MerchantPaymentId`: Represents a key for merchant and payment with merchant ID and payment ID.
//! - `Whconf`: Represents a key for webhook configurations with merchant ID and connector ID.
//! - `WhSecVerification`: Represents a key for webhook security verification with connector label and merchant ID.
//!
//! The `RedisKey` enum implements the `ToString` trait, allowing conversion to a string representation of the Redis key.
//!
//! # Examples
//!
//! ```rust
//! let payment_key = RedisKey::PaymentId { payment_id: "67890" };
//! assert_eq!(payment_key.to_string(), "pi_67890");
//!
//! let creds_key = RedisKey::McdCredsId {
//!     merchant_id: "54321",
//!     creds_identifier: "cred_123",
//! };
//! assert_eq!(creds_key.to_string(), "mcd_54321_cred_123");
//! ```
//!
#[derive(Debug)]
#[allow(missing_docs)]
/// Create
pub enum RedisKey<'a> {
    /// for "access_token_{merchant_id}_{connector_name}"
    AccsesToken {
        merchant_id: &'a str,
        connector_name: &'a str,
    },
    /// for "mcd_{merchant_id}_{creds_identifier}"
    McdCredsId {
        merchant_id: &'a str,
        creds_identifier: &'a str,
    },
    /// for "connector_resp_{merchant_id}_{payment_id}_{attempt_id}"
    ConnectorResponse {
        merchant_id: &'a str,
        payment_id: &'a str,
        attempt_id: &'a str,
    },
    /// for "pi_{payment_id}"
    PaymentId { payment_id: &'a str },
    /// for "mid_{merchant_id}_pid_{payment_id}"
    MerchantPaymentId {
        merchant_id: &'a str,
        payment_id: &'a str,
    },
    /// for "whconf_{merchant_id}_{connector_id}"
    Whconf {
        merchant_id: &'a str,
        connector_id: &'a str,
    },
    /// for "whsec_verification_{connector_label}_{merchant_id}"
    WhSecVerification {
        connector_label: &'a str,
        merchant_id: &'a str,
    },
}

/// converts the redis key to string
impl ToString for RedisKey<'_> {
    fn to_string(&self) -> String {
        match self {
            Self::AccsesToken {
                merchant_id,
                connector_name,
            } => {
                format!("access_token_{merchant_id}_{connector_name}")
            }
            Self::ConnectorResponse {
                merchant_id,
                payment_id,
                attempt_id,
            } => {
                format!("connector_resp_{merchant_id}_{payment_id}_{attempt_id}")
            }
            Self::McdCredsId {
                merchant_id,
                creds_identifier,
            } => {
                format!("mcd_{merchant_id}_{creds_identifier}")
            }
            Self::PaymentId { payment_id } => {
                format!("pi_{payment_id}")
            }
            Self::MerchantPaymentId {
                merchant_id,
                payment_id,
            } => {
                format!("mid_{merchant_id}_pid_{payment_id}")
            }
            Self::Whconf {
                merchant_id,
                connector_id,
            } => {
                format!("whconf_{merchant_id}_{connector_id}")
            }
            Self::WhSecVerification {
                connector_label,
                merchant_id,
            } => {
                format!("whsec_verification_{connector_label}_{merchant_id}")
            }
        }
    }
}
