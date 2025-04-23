use common_utils::types::MinorUnit;
use error_stack::Report;
use hyperswitch_domain_models::router_data::ConnectorAuthType;
use hyperswitch_interfaces::errors::ConnectorError;
use masking::Secret;
use serde::Serialize;

#[cfg(feature = "payouts")]
pub mod payouts;
#[cfg(feature = "payouts")]
pub use payouts::*;

// Error signature
type Error = Report<ConnectorError>;

// Auth Struct
pub struct AdyenplatformAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for AdyenplatformAuthType {
    type Error = Error;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AdyenPlatformRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

impl<T> TryFrom<(MinorUnit, T)> for AdyenPlatformRouterData<T> {
    type Error = Report<ConnectorError>;
    fn try_from((amount, item): (MinorUnit, T)) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}
