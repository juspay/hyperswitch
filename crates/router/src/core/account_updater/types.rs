use std::{collections::HashSet, time::Duration};

use common_enums::{connector_enums::Connector, CardNetwork};
use error_stack::ResultExt;
use hyperswitch_domain_models::router_data::ConnectorAuthType;
use hyperswitch_masking::Secret;

use crate::{
    configs::settings,
    core::{
        errors::{self, RouterResult},
        unified_connector_service::connector_config::JuspayMetadata,
    },
};

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
    Juspay(JuspayConfig),
}

impl ResolvedAccountUpdaterConfig {
    pub fn supported_card_networks(&self) -> &HashSet<CardNetwork> {
        match self {
            Self::Juspay(juspay) => &juspay.supported_card_networks,
        }
    }

    pub fn refresh_timeout(&self) -> Duration {
        match self {
            Self::Juspay(juspay) => juspay.refresh_timeout,
        }
    }

    /// Builds the connector, auth type and metadata the connector config is resolved from.
    pub fn build_connector_credentials(
        &self,
    ) -> RouterResult<(Connector, ConnectorAuthType, serde_json::Value)> {
        match self {
            Self::Juspay(juspay) => {
                let metadata = serde_json::to_value(JuspayMetadata {
                    merchant_id: juspay.merchant_id.clone(),
                    base_url: juspay.base_url.to_string(),
                    juspay_encryption_public_key: juspay.euler_encryption_public_key.clone(),
                    response_decryption_private_key: juspay.au_decryption_pvt_key.clone(),
                    card_sync_key_id: juspay.card_sync_key_id.clone(),
                })
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to serialize the Juspay Account Updater metadata")?;

                Ok((
                    Connector::Juspay,
                    ConnectorAuthType::HeaderKey {
                        api_key: juspay.api_key.clone(),
                    },
                    metadata,
                ))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct JuspayConfig {
    pub base_url: url::Url,
    pub api_key: Secret<String>,
    pub merchant_id: String,
    pub euler_encryption_public_key: Secret<String>,
    pub au_decryption_pvt_key: Secret<String>,
    pub card_sync_key_id: String,
    pub supported_card_networks: HashSet<CardNetwork>,
    pub refresh_timeout: Duration,
}

impl From<&settings::JuspayAccountUpdaterConfig> for JuspayConfig {
    fn from(config: &settings::JuspayAccountUpdaterConfig) -> Self {
        Self {
            base_url: config.base_url.clone(),
            api_key: config.api_key.clone(),
            merchant_id: config.merchant_id.clone(),
            euler_encryption_public_key: config.euler_encryption_public_key.clone(),
            au_decryption_pvt_key: config.au_decryption_pvt_key.clone(),
            card_sync_key_id: config.card_sync_key_id.clone(),
            supported_card_networks: config.supported_card_networks.clone(),
            refresh_timeout: Duration::from_secs(config.refresh_timeout_in_secs),
        }
    }
}

impl From<&settings::AccountUpdaterConfig> for ResolvedAccountUpdaterConfig {
    fn from(config: &settings::AccountUpdaterConfig) -> Self {
        match config {
            settings::AccountUpdaterConfig::Juspay(juspay) => Self::Juspay(juspay.into()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AccountUpdaterError {
    #[error("Account Updater application config is missing or invalid")]
    MissingApplicationConfig,
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
