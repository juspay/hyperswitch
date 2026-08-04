use hyperswitch_masking::Secret;
use router_env::logger;

use super::types::{AccountUpdaterFailure, ResolvedAccountUpdaterConfig};
use crate::core::unified_connector_service::connector_config::{
    serialize_connector_config, ConnectorSpecificConfig,
};

pub fn build_account_updater_connector_config(
    config: &ResolvedAccountUpdaterConfig,
) -> Result<Secret<String>, AccountUpdaterFailure> {
    let connector_config = match config {
        ResolvedAccountUpdaterConfig::Juspay(juspay) => ConnectorSpecificConfig::Juspay {
            api_key: juspay.api_key.clone(),
            merchant_id: juspay.merchant_id.clone(),
            base_url: juspay.base_url.to_string(),
            juspay_encryption_public_key: juspay.euler_encryption_public_key.clone(),
            response_decryption_private_key: juspay.au_decryption_pvt_key.clone(),
            card_sync_key_id: juspay.card_sync_key_id.clone(),
        },
    };

    serialize_connector_config(&connector_config)
        .map(Secret::new)
        .map_err(|error| {
            logger::warn!(
                ?error,
                "Failed to serialize the Account Updater connector config"
            );
            AccountUpdaterFailure::RefreshCallFailed
        })
}
