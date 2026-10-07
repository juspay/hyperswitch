use crate::core::payments::helpers;

/// Provider-profile vault configuration, separate from payout connector credentials and tokens.
#[derive(Clone, Debug, Default)]
pub enum PayoutExecutionContext {
    #[default]
    Normal,
    ExternalVaultProxy {
        external_vault_mca: helpers::MerchantConnectorAccountType,
    },
}
