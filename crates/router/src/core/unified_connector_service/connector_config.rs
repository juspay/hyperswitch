//! Connector-specific configuration transformer for UCS
//!
//! This module provides functionality to transform connector authentication data
//! into connector-specific configuration structures expected by the Unified Connector Service (UCS).

use std::str::FromStr;

use common_enums::connector_enums::Connector;
use error_stack::ResultExt;
use hyperswitch_domain_models::router_data::ConnectorAuthType;
use masking::Secret;
use serde::Serialize;

use super::super::errors::RouterResult;

/// Connector-specific configuration wrapper for UCS.
/// Serializes as: `{"config": {"ConnectorName": {...}}}`
#[derive(Debug, Serialize)]
pub struct UcsConnectorConfig {
    pub config: serde_json::Map<String, serde_json::Value>,
}

impl UcsConnectorConfig {
    /// Creates a new UCS connector config with the connector name as the key in PascalCase
    pub fn new<T: Serialize>(connector: Connector, inner: T) -> RouterResult<Self> {
        let connector_name = format!("{:?}", connector); // PascalCase: Braintree, Cybersource
        let inner_json = serde_json::to_value(&inner)
            .change_context(super::super::errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to serialize connector config inner value")?;
        let mut config = serde_json::Map::new();
        config.insert(connector_name, inner_json);
        Ok(Self { config })
    }
}

/// Cybersource-specific connector configuration
#[derive(Debug, Serialize)]
pub struct CybersourceConfig {
    pub api_key: Secret<String>,
    pub merchant_account: Secret<String>,
    pub api_secret: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_avs: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_cvn: Option<bool>,
}

/// Braintree-specific connector configuration
#[derive(Debug, Serialize)]
pub struct BraintreeConfig {
    pub public_key: Secret<String>,
    pub private_key: Secret<String>,
    pub key1: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merchant_account_id: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merchant_config_currency: Option<String>,
}

/// Build the X_CONNECTOR_CONFIG header value for any connector
pub fn build_connector_config_header(
    connector_name: &str,
    auth_type: &ConnectorAuthType,
    connector_metadata: Option<&serde_json::Value>,
    base_url: Option<String>,
) -> RouterResult<String> {
    let connector = Connector::from_str(connector_name)
        .change_context(super::super::errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| format!("Invalid connector name: {}", connector_name))?;

    let config = match connector {
        Connector::Cybersource => {
            build_cybersource_config(auth_type, connector_metadata, base_url)?
        }
        Connector::Braintree => build_braintree_config(auth_type, connector_metadata, base_url)?,
        _ => {
            return Err(error_stack::report!(
                super::super::errors::ApiErrorResponse::InternalServerError
            )
            .attach_printable(format!(
                "Connector {} not yet supported for ConnectorSpecificConfig",
                connector
            )));
        }
    };

    serde_json::to_string(&config)
        .change_context(super::super::errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialize ConnectorSpecificConfig")
}

fn build_cybersource_config(
    auth_type: &ConnectorAuthType,
    connector_metadata: Option<&serde_json::Value>,
    base_url: Option<String>,
) -> RouterResult<UcsConnectorConfig> {
    let (api_key, merchant_account, api_secret) = match auth_type {
        ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } => (api_key.clone(), key1.clone(), api_secret.clone()),
        _ => {
            return Err(error_stack::report!(
                super::super::errors::ApiErrorResponse::InternalServerError
            )
            .attach_printable("Unsupported auth type for Cybersource ConnectorSpecificConfig"));
        }
    };

    #[derive(Debug, serde::Deserialize)]
    struct CybersourceMetadata {
        disable_avs: Option<bool>,
        disable_cvn: Option<bool>,
    }

    let (disable_avs, disable_cvn) = connector_metadata
        .and_then(|m| serde_json::from_value::<CybersourceMetadata>(m.clone()).ok())
        .map(|m| (m.disable_avs, m.disable_cvn))
        .unwrap_or((None, None));

    let cybersource_config = CybersourceConfig {
        api_key,
        merchant_account,
        api_secret,
        base_url,
        disable_avs,
        disable_cvn,
    };

    UcsConnectorConfig::new(Connector::Cybersource, cybersource_config)
}

fn build_braintree_config(
    auth_type: &ConnectorAuthType,
    connector_metadata: Option<&serde_json::Value>,
    base_url: Option<String>,
) -> RouterResult<UcsConnectorConfig> {
    let (public_key, private_key, key1) = match auth_type {
        ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } => (api_key.clone(), api_secret.clone(), key1.clone()),
        _ => {
            return Err(error_stack::report!(
                super::super::errors::ApiErrorResponse::InternalServerError
            )
            .attach_printable("Unsupported auth type for Braintree ConnectorSpecificConfig"));
        }
    };

    #[derive(Debug, serde::Deserialize)]
    struct BraintreeMetadata {
        merchant_account_id: Option<Secret<String>>,
        merchant_config_currency: Option<String>,
    }

    let (merchant_account_id, merchant_config_currency) = connector_metadata
        .and_then(|m| serde_json::from_value::<BraintreeMetadata>(m.clone()).ok())
        .map(|m| (m.merchant_account_id, m.merchant_config_currency))
        .unwrap_or((None, None));

    let braintree_config = BraintreeConfig {
        public_key,
        private_key,
        key1,
        base_url,
        merchant_account_id,
        merchant_config_currency,
    };

    UcsConnectorConfig::new(Connector::Braintree, braintree_config)
}
