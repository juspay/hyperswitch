//! Request-scoped vault tokens; never persist them in payout records.

#[derive(Clone, Debug)]
pub enum ExternalVaultPayoutMethodData {
    Card(Box<ExternalVaultPayoutCardData>),
}

/// Opaque vault tokens bypass PAN validation and local detokenization.
#[derive(Clone, Debug)]
pub struct ExternalVaultPayoutCardData {
    pub card_number: hyperswitch_masking::Secret<String>,
    pub expiry_month: hyperswitch_masking::Secret<String>,
    pub expiry_year: hyperswitch_masking::Secret<String>,
    pub card_holder_name: Option<hyperswitch_masking::Secret<String>>,
    pub card_network: Option<common_enums::CardNetwork>,
}
