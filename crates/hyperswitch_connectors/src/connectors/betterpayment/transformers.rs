use hyperswitch_domain_models::router_data::ConnectorAuthType;
use hyperswitch_interfaces::errors;

pub struct BetterpaymentAuthType;

impl TryFrom<&ConnectorAuthType> for BetterpaymentAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { .. } => Ok(Self),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)?,
        }
    }
}
