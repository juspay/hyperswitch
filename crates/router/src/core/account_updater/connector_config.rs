use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use hyperswitch_masking::Secret;

use super::types::{AccountUpdaterError, ResolvedAccountUpdaterConfig};

#[derive(Debug, serde::Serialize)]
struct JuspayConnectorConfig {
    api_key: Secret<String>,
    merchant_id: String,
    base_url: String,
    #[serde(rename = "juspay_encryption_public_key")]
    euler_encryption_public_key: Secret<String>,
    #[serde(rename = "response_decryption_private_key")]
    au_decryption_pvt_key: Secret<String>,
    card_sync_key_id: Secret<String>,
}

#[derive(Debug, serde::Serialize)]
enum AccountUpdaterConnectorConfig {
    Juspay(JuspayConnectorConfig),
}

pub fn build_account_updater_connector_config(
    config: &ResolvedAccountUpdaterConfig,
) -> CustomResult<Secret<String>, AccountUpdaterError> {
    let connector_config = AccountUpdaterConnectorConfig::Juspay(JuspayConnectorConfig {
        api_key: config.api_key.clone(),
        merchant_id: config.merchant_id.clone(),
        base_url: config.base_url.to_string(),
        euler_encryption_public_key: config.euler_encryption_public_key.clone(),
        au_decryption_pvt_key: config.au_decryption_pvt_key.clone(),
        card_sync_key_id: config.card_sync_key_id.clone(),
    });

    let config_json = serde_json::to_value(&connector_config)
        .change_context(AccountUpdaterError::ConnectorConfigSerializationFailed)?;

    let mut outer_map = serde_json::Map::new();
    outer_map.insert("config".to_string(), config_json);

    serde_json::to_string(&outer_map)
        .map(Secret::new)
        .change_context(AccountUpdaterError::ConnectorConfigSerializationFailed)
}

#[cfg(test)]
mod tests {
    use hyperswitch_masking::PeekInterface;

    use super::*;

    fn resolved_config() -> ResolvedAccountUpdaterConfig {
        ResolvedAccountUpdaterConfig {
            base_url: url::Url::parse("https://sandbox.example.com/").unwrap(),
            api_key: Secret::new("api_key".to_string()),
            merchant_id: "merchant".to_string(),
            euler_encryption_public_key: Secret::new("public_key".to_string()),
            au_decryption_pvt_key: Secret::new("private_key".to_string()),
            card_sync_key_id: Secret::new("key_id".to_string()),
        }
    }

    #[test]
    fn serializes_key_fields_under_the_names_ucs_expects() {
        let header = build_account_updater_connector_config(&resolved_config()).unwrap();
        let value: serde_json::Value = serde_json::from_str(header.peek()).unwrap();
        let juspay = &value["config"]["Juspay"];

        assert_eq!(juspay["juspay_encryption_public_key"], "public_key");
        assert_eq!(juspay["response_decryption_private_key"], "private_key");
        assert_eq!(juspay["card_sync_key_id"], "key_id");
        assert_eq!(juspay["api_key"], "api_key");
        assert_eq!(juspay["merchant_id"], "merchant");
        assert_eq!(juspay["base_url"], "https://sandbox.example.com/");

        assert!(juspay.get("euler_encryption_public_key").is_none());
        assert!(juspay.get("au_decryption_pvt_key").is_none());
    }
}
