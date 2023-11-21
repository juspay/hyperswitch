//! This module defines a Rust enum `RedisKey` that facilitates the creation of Redis keys for various purposes.

#[derive(Debug)]
#[allow(missing_docs)]
/// This enum facilitates the creation of Redis keys for various purposes.
/// The `RedisKey` enum implements the `ToString` trait, allowing conversion to a string representation of the Redis key.
///
/// # Example
///
/// ```rust
/// let payment_key = RedisKey::PaymentId { payment_id: "67890" };
/// assert_eq!(payment_key.to_string(), "pi_67890");
///
/// let creds_key = RedisKey::McdCredsId {
///     merchant_id: "54321",
///     creds_identifier: "cred_123",
/// };
/// assert_eq!(creds_key.to_string(), "mcd_54321_cred_123");
/// ```
///
pub enum RedisKey<'a> {
    /// for "access_token_{merchant_id}_{connector_name}"
    AccessToken {
        merchant_id: &'a str,
        connector_name: &'a str,
    },
    /// for "mcd_{merchant_id}_{creds_identifier}"
    McdCredsId {
        merchant_id: &'a str,
        creds_identifier: &'a str,
    },
    /// for "pi_{payment_id}"
    PaymentId { payment_id: &'a str },
    /// for "mid_{merchant_id}_pid_{payment_id}"
    MerchantPaymentId {
        merchant_id: &'a str,
        payment_id: &'a str,
    },
    /// for "whconf_{merchant_id}_{connector_id}"
    WhConf {
        merchant_id: &'a str,
        connector_id: &'a str,
    },
    /// for "whconf_disabled_events_{merchant_id}_{connector_id}"
    WhConfDisabledEvents {
        merchant_id: &'a str,
        connector_id: &'a str,
    },
}

/// converts the redis key to string
impl ToString for RedisKey<'_> {
    fn to_string(&self) -> String {
        match self {
            Self::AccessToken {
                merchant_id,
                connector_name,
            } => format!("access_token_{merchant_id}_{connector_name}"),
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
            Self::WhConf {
                merchant_id,
                connector_id,
            } => {
                format!("whconf_{merchant_id}_{connector_id}")
            }
            Self::WhConfDisabledEvents {
                merchant_id,
                connector_id,
            } => {
                format!("whconf_disabled_events_{merchant_id}_{connector_id}")
            }
        }
    }
}
