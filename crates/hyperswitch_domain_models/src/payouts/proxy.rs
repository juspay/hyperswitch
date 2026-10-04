//! Runtime-only external-vault payout data. Do not persist these tokens in payout records.

use common_enums::CardNetwork;
use hyperswitch_masking::Secret;

#[derive(Clone, Debug)]
pub enum ExternalVaultPayoutMethodData {
    Card(Box<ExternalVaultPayoutCardData>),
}

/// Opaque vault values used to construct a connector request template. The card
/// number is a token, so it must not undergo PAN validation or local detokenization.
#[derive(Clone, Debug)]
pub struct ExternalVaultPayoutCardData {
    pub card_number: Secret<String>,
    pub expiry_month: Secret<String>,
    pub expiry_year: Secret<String>,
    pub card_holder_name: Option<Secret<String>>,
    pub card_network: Option<CardNetwork>,
}
