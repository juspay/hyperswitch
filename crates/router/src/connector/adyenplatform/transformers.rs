use error_stack::Report;
use masking::Secret;

#[cfg(feature = "payouts")]
pub mod payouts;
#[cfg(feature = "payouts")]
pub use payouts::*;

use crate::{core::errors, types};

// Error signature
type Error = Report<errors::ConnectorError>;

// Auth Struct
pub struct AdyenplatformAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for AdyenplatformAuthType {
    type Error = Error;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
