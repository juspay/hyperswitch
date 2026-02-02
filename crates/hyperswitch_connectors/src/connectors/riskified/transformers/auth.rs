use hyperswitch_domain_models::router_data::ConnectorAuthType;
use hyperswitch_interfaces::errors::ConnectorError;
use masking::{ExposeInterface, Secret};

pub struct RiskifiedAuthType {
    pub secret_token: Secret<String>,
    pub domain_name: String,
}

impl TryFrom<&ConnectorAuthType> for RiskifiedAuthType {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                secret_token: api_key.to_owned(),
                domain_name: key1.to_owned().expose(),
            }),
            _ => Err(ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
