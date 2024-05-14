use pm_auth::types::{self as pm_auth_types};

use crate::{core::errors, types::transformers::ForeignTryFrom};

impl ForeignTryFrom<hyperswitch_domain_models::router_data::ConnectorAuthType>
    for pm_auth_types::ConnectorAuthType
{
    type Error = errors::ConnectorError;
    fn foreign_try_from(
        auth_type: hyperswitch_domain_models::router_data::ConnectorAuthType,
    ) -> Result<Self, Self::Error> {
        match auth_type {
            hyperswitch_domain_models::router_data::ConnectorAuthType::BodyKey {
                api_key,
                key1,
            } => Ok::<Self, errors::ConnectorError>(Self::BodyKey {
                client_id: api_key.to_owned(),
                secret: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType),
        }
    }
}
