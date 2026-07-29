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
    pub card_sync_key_id: Secret<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum AccountUpdaterError {
    #[error("Account Updater application config is missing: {0}")]
    MissingApplicationConfig(String),
    #[error("Failed to serialize the Account Updater connector config")]
    ConnectorConfigSerializationFailed,
}
