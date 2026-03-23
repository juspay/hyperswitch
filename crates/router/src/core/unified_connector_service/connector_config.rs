//! Connector-specific configuration transformer for UCS
//!
//! This module provides functionality to transform connector authentication data
//! into connector-specific configuration structures expected by the Unified Connector Service (UCS).

use std::str::FromStr;

use common_enums::connector_enums::Connector;
use error_stack::ResultExt;
use hyperswitch_domain_models::router_data::ConnectorAuthType;
use hyperswitch_masking::Secret;
use serde::Serialize;

use crate::{
    core::errors::{self, RouterResult},
    types::transformers::ForeignTryFrom,
};

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
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to serialize connector config inner value")?;
        let mut config = serde_json::Map::new();
        config.insert(connector_name, inner_json);
        Ok(Self { config })
    }
}

/// Metadata structures for parsing connector metadata
#[derive(Debug, serde::Deserialize)]
pub struct CybersourceMetadata {
    disable_avs: Option<bool>,
    disable_cvn: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
pub struct BraintreeMetadata {
    merchant_account_id: Option<Secret<String>>,
    merchant_config_currency: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct AdyenMetadata {
    endpoint_prefix: Option<Secret<String>>,
}

#[derive(Debug, serde::Deserialize)]
pub struct TruelayerMetadata {
    merchant_account_id: Option<Secret<String>>,
    account_holder_name: Option<Secret<String>>,
    private_key: Option<Secret<String>>,
    kid: Option<Secret<String>>,
}

/// Connector-specific configuration enum for all supported connectors
#[derive(Debug, Clone, serde::Serialize)]
pub enum ConnectorSpecificConfig {
    /// Adyen connector configuration
    Adyen {
        api_key: Secret<String>,
        merchant_account: Secret<String>,
        review_key: Option<Secret<String>>,
        endpoint_prefix: Option<Secret<String>>,
    },
    /// Braintree connector configuration
    Braintree {
        public_key: Secret<String>,
        private_key: Secret<String>,
        merchant_account_id: Secret<String>,
        merchant_config_currency: Option<String>,
    },
    /// Cybersource connector configuration
    Cybersource {
        api_key: Secret<String>,
        merchant_account: Secret<String>,
        api_secret: Secret<String>,
        disable_avs: Option<bool>,
        disable_cvn: Option<bool>,
    },
    /// Stripe connector configuration
    Stripe { api_key: Secret<String> },
    /// PayPal connector configuration
    Paypal {
        client_id: Secret<String>,
        client_secret: Secret<String>,
        payer_id: Option<Secret<String>>,
    },
    /// Truelayer connector configuration
    Truelayer {
        client_id: Secret<String>,
        client_secret: Secret<String>,
        merchant_account_id: Option<Secret<String>>,
        account_holder_name: Option<Secret<String>>,
        private_key: Option<Secret<String>>,
        kid: Option<Secret<String>>,
    },
    /// Revolv3 connector configuration
    Revolv3 { api_key: Secret<String> },
    /// Fiservcommercehub connector configuration
    Fiservcommercehub {
        api_key: Secret<String>,
        secret: Secret<String>,
        merchant_id: Secret<String>,
        terminal_id: Secret<String>,
    },
    /// Trustly connector configuration
    Trustly {
        username: Secret<String>,
        password: Secret<String>,
        private_key: Secret<String>,
    },
}

impl ForeignTryFrom<(Connector, &ConnectorAuthType, Option<&serde_json::Value>)>
    for ConnectorSpecificConfig
{
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn foreign_try_from(
        (connector, auth, metadata): (Connector, &ConnectorAuthType, Option<&serde_json::Value>),
    ) -> Result<Self, Self::Error> {
        let err = |msg: &str| {
            error_stack::report!(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(msg.to_string())
        };

        match connector {
            Connector::Adyen => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => {
                    let endpoint_prefix = metadata
                        .and_then(|m| serde_json::from_value::<AdyenMetadata>(m.clone()).ok())
                        .and_then(|m| m.endpoint_prefix);

                    Ok(Self::Adyen {
                        api_key: api_key.clone(),
                        merchant_account: key1.clone(),
                        review_key: Some(api_secret.clone()),
                        endpoint_prefix,
                    })
                }
                ConnectorAuthType::BodyKey { api_key, key1 } => {
                    let endpoint_prefix = metadata
                        .and_then(|m| serde_json::from_value::<AdyenMetadata>(m.clone()).ok())
                        .and_then(|m| m.endpoint_prefix);

                    Ok(Self::Adyen {
                        api_key: api_key.clone(),
                        merchant_account: key1.clone(),
                        review_key: None,
                        endpoint_prefix,
                    })
                }
                _ => Err(err("Adyen requires SignatureKey or BodyKey auth type")),
            },
            Connector::Cybersource => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => {
                    let (disable_avs, disable_cvn) = metadata
                        .and_then(|m| serde_json::from_value::<CybersourceMetadata>(m.clone()).ok())
                        .map(|m| (m.disable_avs, m.disable_cvn))
                        .unwrap_or((None, None));

                    Ok(Self::Cybersource {
                        api_key: api_key.clone(),
                        merchant_account: key1.clone(),
                        api_secret: api_secret.clone(),
                        disable_avs,
                        disable_cvn,
                    })
                }
                _ => Err(err("Cybersource requires SignatureKey auth type")),
            },
            Connector::Braintree => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => {
                    let metadata_parsed = metadata
                        .and_then(|m| serde_json::from_value::<BraintreeMetadata>(m.clone()).ok());

                    let merchant_account_id = metadata_parsed
                        .as_ref()
                        .and_then(|m| m.merchant_account_id.clone())
                        .unwrap_or_else(|| key1.clone());

                    let merchant_config_currency =
                        metadata_parsed.and_then(|m| m.merchant_config_currency);

                    Ok(Self::Braintree {
                        public_key: api_key.clone(),
                        private_key: api_secret.clone(),
                        merchant_account_id,
                        merchant_config_currency,
                    })
                }
                _ => Err(err("Braintree requires SignatureKey auth type")),
            },
            Connector::Stripe => match auth {
                ConnectorAuthType::HeaderKey { api_key } => Ok(Self::Stripe {
                    api_key: api_key.clone(),
                }),
                _ => Err(err("Stripe requires HeaderKey auth type")),
            },
            Connector::Truelayer => match auth {
                ConnectorAuthType::BodyKey { api_key, key1 } => {
                    let metadata_parsed = metadata
                        .and_then(|m| serde_json::from_value::<TruelayerMetadata>(m.clone()).ok());

                    Ok(Self::Truelayer {
                        client_id: api_key.clone(),
                        client_secret: key1.clone(),
                        merchant_account_id: metadata_parsed
                            .as_ref()
                            .and_then(|m| m.merchant_account_id.clone()),
                        account_holder_name: metadata_parsed
                            .as_ref()
                            .and_then(|m| m.account_holder_name.clone()),
                        private_key: metadata_parsed.as_ref().and_then(|m| m.private_key.clone()),
                        kid: metadata_parsed.as_ref().and_then(|m| m.kid.clone()),
                    })
                }
                _ => Err(err("Truelayer requires BodyKey auth type")),
            },
            Connector::Paypal => match auth {
                ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self::Paypal {
                    client_id: key1.clone(),
                    client_secret: api_key.clone(),
                    payer_id: None,
                }),
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Paypal {
                    client_id: key1.clone(),
                    client_secret: api_key.clone(),
                    payer_id: Some(api_secret.clone()),
                }),
                _ => Err(err("Paypal requires BodyKey or SignatureKey auth type")),
            },
            Connector::Trustly => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Trustly {
                    username: api_key.clone(),
                    password: key1.clone(),
                    private_key: api_secret.clone(),
                }),
                _ => Err(err("Trustly requires SignatureKey auth type")),
            },
            Connector::Revolv3 => match auth {
                ConnectorAuthType::HeaderKey { api_key } => Ok(Self::Revolv3 {
                    api_key: api_key.clone(),
                }),
                _ => Err(err("Revolv3 requires HeaderKey auth type")),
            },
            Connector::Fiservcommercehub => match auth {
                ConnectorAuthType::MultiAuthKey {
                    api_key,
                    key1,
                    api_secret,
                    key2,
                } => Ok(Self::Fiservcommercehub {
                    api_key: api_key.clone(),
                    secret: api_secret.clone(),
                    merchant_id: key1.clone(),
                    terminal_id: key2.clone(),
                }),
                _ => Err(err("Fiservcommercehub requires MultiAuthKey auth type")),
            },

            // --- Unsupported connectors ---
            _ => Err(
                error_stack::report!(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(format!(
                        "Connector {} not yet supported for ConnectorSpecificConfig",
                        connector
                    )),
            ),
        }
    }
}

/// Build the X_CONNECTOR_CONFIG header value for any connector
pub fn build_connector_config_header(
    connector_name: &str,
    auth_type: &ConnectorAuthType,
    connector_metadata: Option<&serde_json::Value>,
) -> RouterResult<Option<String>> {
    let connector = Connector::from_str(connector_name)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| format!("Invalid connector name: {}", connector_name))?;

    let config =
        match ConnectorSpecificConfig::foreign_try_from((connector, auth_type, connector_metadata))
        {
            Ok(config) => config,
            Err(_) => {
                // Connector is not supported for specific config - this is not an error,
                // just means no connector-specific config is needed
                return Ok(None);
            }
        };

    let config_json = serde_json::to_value(&config)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialize connector config to JSON value")?;

    let mut outer_map = serde_json::Map::new();
    outer_map.insert("config".to_string(), config_json);

    let config_string = serde_json::to_string(&outer_map)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialize ConnectorSpecificConfig")?;

    Ok(Some(config_string))
}
