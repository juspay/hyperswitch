use common_enums::connector_enums::Connector;
use error_stack::ResultExt;
use hyperswitch_domain_models::router_data::ConnectorAuthType;
use hyperswitch_masking::Secret;

use crate::core::{
    errors::{self, RouterResult},
    unified_connector_service::connector_config::JuspayMetadata,
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
    Juspay(JuspayCredentials),
}

impl ResolvedAccountUpdaterConfig {
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
pub struct JuspayCredentials {
    pub base_url: url::Url,
    pub api_key: Secret<String>,
    pub merchant_id: String,
    pub euler_encryption_public_key: Secret<String>,
    pub au_decryption_pvt_key: Secret<String>,
    pub card_sync_key_id: String,
}

/// The `serde` names are what the observation event records.
#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(rename_all = "snake_case")]
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
