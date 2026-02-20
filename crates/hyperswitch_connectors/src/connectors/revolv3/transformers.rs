use hyperswitch_domain_models::router_data::ConnectorAuthType;
use hyperswitch_interfaces::errors;

pub struct Revolv3AuthType {
    pub api_key: masking::Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for Revolv3AuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
