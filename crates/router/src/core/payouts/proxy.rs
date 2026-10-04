//! Request-scoped configuration for the isolated proxy core implemented in PR 2.

use crate::core::payments::helpers::MerchantConnectorAccountType;

/// Runtime execution configuration, separate from the payout connector MCA and
/// from token-bearing method data. Resolve the vault MCA through the provider's
/// business profile, as payments do; never accept it from merchant metadata.
#[derive(Clone, Default)]
pub enum PayoutExecutionContext {
    #[default]
    Normal,
    ExternalVaultProxy {
        external_vault_mca: MerchantConnectorAccountType,
    },
}
