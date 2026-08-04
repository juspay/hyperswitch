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

#[derive(Debug, Clone, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum ResolvedAccountUpdaterConfig {
    Juspay(JuspayCredentials),
}

impl ResolvedAccountUpdaterConfig {
    pub fn connector_name(&self) -> &'static str {
        self.into()
    }
}

#[derive(Debug, Clone)]
pub struct JuspayCredentials {
    pub base_url: url::Url,
    pub api_key: Secret<String>,
    pub merchant_id: String,
    pub euler_encryption_public_key: Secret<String>,
    pub au_decryption_pvt_key: Secret<String>,
    pub card_sync_key_id: String,
}

#[derive(Debug, Clone, Copy, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum SkipReason {
    GateDisabled,
    CredentialSourceNone,
    PaymentMethodNotACard,
    PaymentMethodNotActive,
    UnsupportedNetwork,
    CardUnusable,
}

#[derive(Debug)]
pub struct EligibleCard {
    pub network: CardNetwork,
}

#[derive(Debug, Clone, Copy, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum AccountUpdaterFailure {
    RefreshCallFailed,
    RefreshTimedOut,
    RefreshReturnedError,
}

#[derive(Debug, Clone, Copy, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum RefreshOutcome {
    AccountUpdated,
    ExpiryUpdated,
    NoChange,
    Closed,
    NotFound,
    ContactIssuer,
    Unspecified,
}

#[derive(Debug, Clone, Copy)]
pub enum AccountUpdaterTerminalState {
    Skipped(SkipReason),
    Failed(AccountUpdaterFailure),
    Refreshed(RefreshOutcome),
}

impl AccountUpdaterTerminalState {
    pub fn as_labels(&self) -> (&'static str, &'static str) {
        match self {
            Self::Skipped(reason) => ("skipped", reason.into()),
            Self::Failed(failure) => ("failed", failure.into()),
            Self::Refreshed(outcome) => ("refreshed", outcome.into()),
        }
    }
}

impl From<SkipReason> for AccountUpdaterTerminalState {
    fn from(reason: SkipReason) -> Self {
        Self::Skipped(reason)
    }
}

impl From<AccountUpdaterFailure> for AccountUpdaterTerminalState {
    fn from(failure: AccountUpdaterFailure) -> Self {
        Self::Failed(failure)
    }
}

/// Intentionally has no `Debug` impl and no conversion into any response type.
pub struct SyncCard {
    pub card_number: unified_connector_service_cards::CardNumber,
    pub expiry_month: Secret<String>,
    pub expiry_year: Secret<String>,
    pub network: CardNetwork,
}
