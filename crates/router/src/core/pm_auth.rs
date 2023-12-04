use pm_auth::connector::plaid::transformers::PlaidAuthType;

use crate::{
    core::errors,
    types::{self, transformers::ForeignTryFrom},
};

impl ForeignTryFrom<&types::ConnectorAuthType> for PlaidAuthType {
    type Error = errors::ConnectorError;

    fn foreign_try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => {
                Ok::<Self, errors::ConnectorError>(Self {
                    client_id: api_key.to_owned(),
                    secret: key1.to_owned(),
                })
            }
            _ => Err(errors::ConnectorError::FailedToObtainAuthType),
        }
    }
}
