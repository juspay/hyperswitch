use std::collections::HashMap;

use common_utils::pii::SecretSerdeValue;
use error_stack::ResultExt;
use hyperswitch_domain_models::router_data::ConnectorAuthType;
use hyperswitch_interfaces::errors;
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};

use crate::utils;

/// Rapid Connect credentials shared across all payment channels of a merchant connector account.
pub struct RapidconnectAuthType {
    pub group_id: Secret<String>,
    pub terminal_id: Secret<String>,
    pub tpp_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for RapidconnectAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                group_id: api_key.to_owned(),
                terminal_id: key1.to_owned(),
                tpp_id: api_secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

/// Merchant ID and Datawire ID registered for a payment channel. Fiserv issues a separate MID and DID per
/// channel, for example MOTO uses a different MID and DID than ecommerce.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RapidconnectChannelCredentials {
    pub merchant_id: Secret<String>,
    pub datawire_id: Secret<String>,
}

/// Rapid Connect merchant connector account metadata, holding the MID and DID keyed by payment channel
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RapidconnectConnectorMetadataObject {
    pub merchant_details: HashMap<common_enums::PaymentChannel, RapidconnectChannelCredentials>,
}

impl TryFrom<&Option<SecretSerdeValue>> for RapidconnectConnectorMetadataObject {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: &Option<SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "merchant_connector_account.metadata",
            })?;
        if metadata.merchant_details.is_empty() {
            return Err(errors::ConnectorError::InvalidConnectorConfig {
                config: "merchant_details",
            }
            .into());
        }
        Ok(metadata)
    }
}
