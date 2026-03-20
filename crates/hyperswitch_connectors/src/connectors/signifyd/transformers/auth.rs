use hyperswitch_domain_models::router_data::ConnectorAuthType;
use hyperswitch_interfaces::errors::ConnectorError;
use masking::Secret;

pub struct SignifydAuthType {
    pub api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for SignifydAuthType {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
