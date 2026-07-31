use hyperswitch_masking::Secret;
use router_env::logger;

use super::types::{AccountUpdaterFailure, ResolvedAccountUpdaterConfig};

/// Two field names differ from ours: Hyperswitch has no Juspay connector, so `juspay_*` would be a
/// misnomer here. Declared on the fields so a rename on either side breaks in one place.
#[derive(Debug, serde::Serialize)]
struct JuspayConnectorConfig {
    api_key: Secret<String>,
    merchant_id: String,
    base_url: String,
    #[serde(rename = "juspay_encryption_public_key")]
    euler_encryption_public_key: Secret<String>,
    #[serde(rename = "response_decryption_private_key")]
    au_decryption_pvt_key: Secret<String>,
    card_sync_key_id: String,
}

#[derive(Debug, serde::Serialize)]
enum AccountUpdaterConnectorConfig {
    Juspay(JuspayConnectorConfig),
}

#[derive(Debug, serde::Serialize)]
struct AccountUpdaterConnectorConfigEnvelope {
    config: AccountUpdaterConnectorConfig,
}

/// Produces the same `{"config": {"Juspay": {..}}}` header value as
/// `build_connector_config_header`, which cannot be reused here because it derives the header from
/// a `ConnectorAuthType` and these credentials are application-level.
pub fn build_account_updater_connector_config(
    config: &ResolvedAccountUpdaterConfig,
) -> Result<Secret<String>, AccountUpdaterFailure> {
    let envelope = AccountUpdaterConnectorConfigEnvelope {
        config: AccountUpdaterConnectorConfig::Juspay(JuspayConnectorConfig {
            api_key: config.api_key.clone(),
            merchant_id: config.merchant_id.clone(),
            base_url: config.base_url.to_string(),
            euler_encryption_public_key: config.euler_encryption_public_key.clone(),
            au_decryption_pvt_key: config.au_decryption_pvt_key.clone(),
            card_sync_key_id: config.card_sync_key_id.clone(),
        }),
    };

    serde_json::to_string(&envelope)
        .map(Secret::new)
        .map_err(|error| {
            logger::warn!(
                ?error,
                "Failed to serialize the Account Updater connector config"
            );
            AccountUpdaterFailure::ConnectorConfigUnavailable
        })
}
