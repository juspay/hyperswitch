use pm_auth::types as pm_auth_types;

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
            types::MerchantAccountData::FasterPayments {
                account_number,
                sort_code,
                name,
                ..
            } => Self::FasterPayments {
                account_number,
                sort_code,
                name,
            },
            types::MerchantAccountData::Sepa { iban, name, .. } => Self::Sepa { iban, name },
            types::MerchantAccountData::SepaInstant { iban, name, .. } => {
                Self::SepaInstant { iban, name }
            }
            types::MerchantAccountData::Elixir {
                account_number,
                iban,
                name,
                ..
            } => Self::Elixir {
                account_number,
                iban,
                name,
            },
            types::MerchantAccountData::Bankgiro { number, name, .. } => {
                Self::Bankgiro { number, name }
            }
            types::MerchantAccountData::Plusgiro { number, name, .. } => {
                Self::Plusgiro { number, name }
            }
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
            _ => Err(errors::ConnectorError::FailedToObtainAuthType),
        }
    }
}
