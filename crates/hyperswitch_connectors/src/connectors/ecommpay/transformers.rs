use hyperswitch_domain_models::router_data::ConnectorAuthType;
use hyperswitch_interfaces::errors;

/// Ecommpay Gate API v2 credentials.
/// `api_key` = project secret key (HMAC-SHA512 signing key, never transmitted)
/// `key1`    = project id (`general.project_id`)
pub struct EcommpayAuthType {
    pub secret_key: hyperswitch_masking::Secret<String>,
    pub project_id: hyperswitch_masking::Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for EcommpayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                secret_key: api_key.to_owned(),
                project_id: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
