//! Request-scoped configuration for the isolated proxy core implemented in PR 2.

use crate::core::payments::helpers::MerchantConnectorAccountType;

/// Provider-profile vault configuration, separate from payout connector credentials and tokens.
#[derive(Clone, Debug, Default)]
pub enum PayoutExecutionContext {
    #[default]
    Normal,
    ExternalVaultProxy {
        external_vault_mca: MerchantConnectorAccountType,
    },
}
