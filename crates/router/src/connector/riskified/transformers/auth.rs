use error_stack;
use masking::{ExposeInterface, Secret};

use crate::core::errors;

pub struct RiskifiedAuthType {
    pub secret_token: Secret<String>,
    pub domain_name: String,
}

impl TryFrom<&hyperswitch_domain_models::router_data::ConnectorAuthType> for RiskifiedAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        auth_type: &hyperswitch_domain_models::router_data::ConnectorAuthType,
    ) -> Result<Self, Self::Error> {
        match auth_type {
            hyperswitch_domain_models::router_data::ConnectorAuthType::BodyKey {
                api_key,
                key1,
            } => Ok(Self {
                secret_token: api_key.to_owned(),
                domain_name: key1.to_owned().expose(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
