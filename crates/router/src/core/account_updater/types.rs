use common_enums::CardNetwork;
use hyperswitch_masking::Secret;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum AccountUpdaterCredentialSource {
    None,
    Application,
}

#[derive(Debug, Clone)]
pub struct ResolvedAccountUpdaterConfig {
    pub base_url: url::Url,
    pub api_key: Secret<String>,
    pub merchant_id: String,
    pub euler_encryption_public_key: Secret<String>,
    pub au_decryption_pvt_key: Secret<String>,
    pub card_sync_key_id: String,
    pub refresh_timeout_ms: u64,
}

#[derive(Debug)]
pub enum AccountUpdaterGateDecision {
    Proceed(ResolvedAccountUpdaterConfig),
    Skipped(SkipReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SkipReason {
    GateDisabled,
    CredentialSourceNone,
    CredentialsUnavailable,
    NotACard,
    PaymentMethodNotActive,
    UnsupportedNetwork,
    NetworkUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EligibleCard {
    pub network: CardNetwork,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum AccountUpdaterFailure {
    RawCardUnavailable,
    ConnectorConfigUnavailable,
    UnifiedConnectorServiceUnavailable,
    RefreshCallFailed,
    RefreshReturnedError,
    RefreshResultMissing,
}

/// Fieldless by design: the verdict is recorded and the card discarded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum RefreshOutcome {
    AccountUpdated,
    ExpiryUpdated,
    NoChange,
    Closed,
    NotFound,
    ContactIssuer,
    /// The provider answered with a code we do not map.
    Unspecified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountUpdaterTerminalState {
    Skipped(SkipReason),
    Failed(AccountUpdaterFailure),
    Refreshed(RefreshOutcome),
}

/// No `Debug`, and no conversion into any response type: both are deliberate.
pub struct SyncCard {
    pub card_number: cards::CardNumber,
    pub expiry_month: Secret<String>,
    pub expiry_year: Secret<String>,
    pub network: CardNetwork,
}
