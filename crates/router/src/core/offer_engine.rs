pub mod amount;
pub mod apply;
pub mod browse;
pub mod client;
pub mod config;
pub mod connectivity;
pub mod eligibility;
#[cfg(feature = "v1")]
pub mod notify;
pub mod types;
#[cfg(feature = "v1")]
pub mod velocity;

pub use client::OfferEngineClient;
pub use config::resolve_offer_engine_credential_source;

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumString)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum SupportedPaymentMethodType {
    Card,
}

/// Longest card BIN Offer Engine accepts. It matches any prefix from 6 digits up to the length
/// we send, so sending the longest BIN lets 7-9 digit BIN filters match too.
pub const MAX_CARD_BIN_LENGTH: usize = 9;

pub fn is_supported_payment_method_type(payment_method_type: &str) -> bool {
    payment_method_type
        .parse::<SupportedPaymentMethodType>()
        .is_ok()
}
#[cfg(feature = "v1")]
pub use notify::{schedule_payment_notification_for_attempt, schedule_refund_notification};
pub use types::{OfferEngineCredentialSource, OfferEngineError, ResolvedOfferEngineConfig};
