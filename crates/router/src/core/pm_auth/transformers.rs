use pm_auth::types::{self as pm_auth_types};

use crate::{core::errors, types, types::transformers::ForeignTryFrom};

impl ForeignTryFrom<types::ConnectorAuthType> for pm_auth_types::ConnectorAuthType {
    type Error = errors::ConnectorError;
    fn foreign_try_from(auth_type: types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => {
                Ok::<Self, errors::ConnectorError>(Self::BodyKey {
                    client_id: api_key.to_owned(),
                    secret: key1.to_owned(),
                })
            }
            _ => Err(errors::ConnectorError::FailedToObtainAuthType),
        }
    }
}
