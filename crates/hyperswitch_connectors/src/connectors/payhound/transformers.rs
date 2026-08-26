use hyperswitch_domain_models::router_data::ConnectorAuthType;
use hyperswitch_interfaces::errors;

pub struct PayhoundAuthType {
    pub api_key: hyperswitch_masking::Secret<String>,
    pub key1: hyperswitch_masking::Secret<String>,
    pub api_secret: hyperswitch_masking::Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for PayhoundAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                api_key: api_key.to_owned(),
                key1: key1.to_owned(),
                api_secret: api_secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
