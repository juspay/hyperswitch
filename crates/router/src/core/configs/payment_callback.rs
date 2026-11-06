//! Payment callback policy stored as JSON under the `payment_callback` configs key.
//!
//! Example value: `{"mode":"migration"}`.
//! Use the configs API to update this entry so cached values are invalidated.
//! For a staged rollout, explicitly select legacy mode, then switch to migration
//! after all replicas support tokenized paths and enforce once older flows drain.
//! Every tenant must have a valid entry before the HTTP server starts. Tokens expire after
//! `common_utils::consts::DEFAULT_INTENT_FULFILLMENT_TIME` seconds.

use error_stack::ResultExt;
use hyperswitch_domain_models::configs::ConfigInterface;
use serde::Deserialize;
use storage_impl::errors::StorageError;

use crate::{
    core::errors::{self, RouterResult},
    routes::{AppState, SessionState},
};

/// Runtime rollout policy for single-use payment callback URLs.
#[derive(Debug, Deserialize, Clone)]
pub struct PaymentCallbackConfig {
    pub mode: PaymentCallbackMode,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PaymentCallbackMode {
    /// Preserve original URLs until all replicas support tokenized paths.
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

    pub(crate) async fn validate_at_startup(state: &AppState) -> RouterResult<()> {
        for (tenant_id, store) in &state.stores {
            Self::load_from_store(store.as_ref())
                .await
                .attach_printable_lazy(|| {
                    format!(
                        "Invalid or missing payment_callback config for tenant {}",
                        tenant_id.get_string_repr()
                    )
                })?;
        }
        Ok(())
    }

    pub(crate) async fn load(state: &SessionState) -> RouterResult<Self> {
        Self::load_from_store(state.store.as_ref()).await
    }

    async fn load_from_store(
        store: &(impl ConfigInterface<Error = StorageError> + ?Sized),
    ) -> RouterResult<Self> {
        let config = store
            .find_config_by_key_optional(Self::KEY)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to fetch payment_callback config")?;
        Self::parse(config.as_ref().map(|config| config.config.as_str()))
    }

    fn parse(value: Option<&str>) -> RouterResult<Self> {
        let value = value
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Required config payment_callback is missing")?;
        serde_json::from_str(value)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "Invalid payment_callback config: mode must be legacy, migration, or enforced",
            )
    }
}

#[cfg(test)]
mod tests;
