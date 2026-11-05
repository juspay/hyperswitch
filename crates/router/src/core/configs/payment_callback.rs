//! Payment callback policy stored as JSON under the `payment_callback` configs key.
//!
//! Example value: `{"mode":"migration"}`.
//! Use the configs API to update this entry so cached values are invalidated.
//! Deploy support in legacy mode, switch to migration after all replicas support
//! tokenized paths, then enforce once older payment flows have drained.
//! A missing entry defaults to legacy mode. Tokens expire after
//! `common_utils::consts::DEFAULT_INTENT_FULFILLMENT_TIME` seconds.

use error_stack::ResultExt;
use serde::Deserialize;

use crate::{
    core::errors::{self, RouterResult},
    routes::SessionState,
};

/// Runtime rollout policy for single-use payment callback URLs.
#[derive(Debug, Default, Deserialize, Clone)]
#[serde(default)]
pub struct PaymentCallbackConfig {
    pub mode: PaymentCallbackMode,
}

#[derive(Debug, Deserialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PaymentCallbackMode {
    /// Preserve original URLs until all replicas support tokenized paths.
    #[default]
    Legacy,
    /// Issue tokenized paths while accepting callbacks from older payment flows.
    Migration,
    /// Require a token on every protected payment callback.
    Enforced,
}

impl PaymentCallbackMode {
    pub const fn issues_tokens(self) -> bool {
        !matches!(self, Self::Legacy)
    }

    pub const fn allows_legacy(self) -> bool {
        !matches!(self, Self::Enforced)
    }
}

impl PaymentCallbackConfig {
    pub const KEY: &'static str = "payment_callback";

    pub(crate) async fn load(state: &SessionState) -> RouterResult<Self> {
        let config = state
            .store
            .find_config_by_key_optional(Self::KEY)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to fetch payment callback config")?;
        Self::parse(config.as_ref().map(|config| config.config.as_str()))
    }

    fn parse(value: Option<&str>) -> RouterResult<Self> {
        let config: Self = value
            .map(serde_json::from_str)
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Invalid payment callback config")?
            .unwrap_or_default();
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_callback_config_uses_defaults() {
        for value in [None, Some("{}"), Some(r#"{"mode":"legacy"}"#)] {
            let config = PaymentCallbackConfig::parse(value).expect("default config");
            assert_eq!(config.mode, PaymentCallbackMode::Legacy);
        }
    }

    #[test]
    fn callback_config_supports_each_rollout_mode() {
        for (value, mode) in [
            ("legacy", PaymentCallbackMode::Legacy),
            ("migration", PaymentCallbackMode::Migration),
            ("enforced", PaymentCallbackMode::Enforced),
        ] {
            let value = serde_json::json!({"mode": value});
            let config = PaymentCallbackConfig::parse(Some(&value.to_string()))
                .expect("valid callback config");
            assert_eq!(config.mode, mode);
        }
    }

    #[test]
    fn invalid_callback_config_never_falls_back_to_legacy() {
        for value in [
            "invalid JSON",
            "null",
            r#"{"mode":"unknown"}"#,
            r#"{"mode":null}"#,
            r#"{"mode":1}"#,
        ] {
            assert!(
                PaymentCallbackConfig::parse(Some(value)).is_err(),
                "{value}"
            );
        }
    }
}
