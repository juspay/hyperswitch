use error_stack;
use masking::Secret;

use crate::core::errors;

pub struct SignifydAuthType {
    pub api_key: Secret<String>,
}

impl TryFrom<&hyperswitch_domain_models::router_data::ConnectorAuthType> for SignifydAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        auth_type: &hyperswitch_domain_models::router_data::ConnectorAuthType,
    ) -> Result<Self, Self::Error> {
        match auth_type {
            hyperswitch_domain_models::router_data::ConnectorAuthType::HeaderKey { api_key } => {
                Ok(Self {
                    api_key: api_key.to_owned(),
                })
            }
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
