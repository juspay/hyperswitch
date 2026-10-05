//! Request-scoped vault tokens; never persist them in payout records.

use common_enums::CardNetwork;
use hyperswitch_masking::Secret;

#[derive(Clone, Debug)]
pub enum ExternalVaultPayoutMethodData {
    Card(Box<ExternalVaultPayoutCardData>),
}

/// Opaque vault tokens bypass PAN validation and local detokenization.
#[derive(Clone, Debug)]
pub struct ExternalVaultPayoutCardData {
    pub card_number: Secret<String>,
    pub expiry_month: Secret<String>,
    pub expiry_year: Secret<String>,
    pub card_holder_name: Option<Secret<String>>,
    pub card_network: Option<CardNetwork>,
}
