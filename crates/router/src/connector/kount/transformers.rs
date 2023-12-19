use crate::{core::errors, types};

pub struct KountAuthType {}

impl TryFrom<&types::ConnectorAuthType> for KountAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::NoKey => Ok(KountAuthType {}),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
