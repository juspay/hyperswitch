use pm_auth::types::{self as pm_auth_types};

use crate::{core::errors, types, types::transformers::ForeignTryFrom};

impl From<types::MerchantAccountData> for pm_auth_types::MerchantAccountData {
    fn from(from: types::MerchantAccountData) -> Self {
        match from {
            types::MerchantAccountData::Iban { iban, name, .. } => Self::Iban { iban, name },
            types::MerchantAccountData::Bacs {
                account_number,
                sort_code,
                name,
                ..
            } => Self::Bacs {
                account_number,
                sort_code,
                name,
            },
        }
    }
}

impl From<types::MerchantRecipientData> for pm_auth_types::MerchantRecipientData {
    fn from(value: types::MerchantRecipientData) -> Self {
        match value {
            types::MerchantRecipientData::ConnectorRecipientId(id) => {
                Self::ConnectorRecipientId(id)
            }
            types::MerchantRecipientData::WalletId(id) => Self::WalletId(id),
            types::MerchantRecipientData::AccountData(data) => Self::AccountData(data.into()),
        }
    }
}

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
            types::ConnectorAuthType::OpenBankingAuth {
                api_key,
                key1,
                merchant_data,
            } => Ok::<Self, errors::ConnectorError>(Self::OpenBankingAuth {
                api_key,
                key1,
                merchant_data: merchant_data.into(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType),
        }
    }
}
