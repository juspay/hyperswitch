use common_enums::connector_enums::Connector;
use hyperswitch_domain_models::router_data::ConnectorAuthType;
use hyperswitch_masking::Secret;

use crate::core::unified_connector_service::connector_config::ConnectorSpecificConfig;

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
pub enum ResolvedAccountUpdaterConfig {
    Juspay(JuspayCredentials),
}

impl ResolvedAccountUpdaterConfig {
    /// Credentials come from application config, so the auth type other connectors parse out
    /// of their merchant connector account is built here instead.
    pub fn to_connector_auth(&self) -> (Connector, ConnectorAuthType, ConnectorSpecificConfig) {
        match self {
            Self::Juspay(juspay) => (
                Connector::Juspay,
                ConnectorAuthType::HeaderKey {
                    api_key: juspay.api_key.clone(),
                },
                ConnectorSpecificConfig::Juspay {
                    api_key: juspay.api_key.clone(),
                    merchant_id: juspay.merchant_id.clone(),
                    base_url: juspay.base_url.to_string(),
                    juspay_encryption_public_key: juspay.euler_encryption_public_key.clone(),
                    response_decryption_private_key: juspay.au_decryption_pvt_key.clone(),
                    card_sync_key_id: juspay.card_sync_key_id.clone(),
                },
            ),
        }
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

#[derive(Debug, thiserror::Error)]
pub enum AccountUpdaterError {
    #[error("Account Updater is disabled")]
    GateDisabled,
    #[error("Account Updater has no credential source configured")]
    CredentialSourceNone,
    #[error("Payment method is not a card")]
    PaymentMethodNotACard,
    #[error("Payment method is not active")]
    PaymentMethodNotActive,
    #[error("Card network is not supported by Account Updater")]
    UnsupportedNetwork,
    #[error("Stored card cannot be used for Account Updater")]
    CardUnusable,
    #[error("Account Updater refresh call failed")]
    RefreshCallFailed,
    #[error("Account Updater refresh returned an error")]
    RefreshReturnedError,
}
