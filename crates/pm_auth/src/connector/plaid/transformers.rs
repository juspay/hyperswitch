use std::collections::HashMap;

use common_enums::{PaymentMethod, PaymentMethodType};
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{core::errors, types};

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct PlaidLinkTokenRequest {
    client_name: String,
    country_codes: Vec<String>,
    language: String,
    products: Vec<String>,
    user: User,
}

#[derive(Debug, Serialize, Eq, PartialEq)]

pub struct User {
    pub client_user_id: String,
}

impl TryFrom<&types::LinkTokenRouterData> for PlaidLinkTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::LinkTokenRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            client_name: item.request.client_name.clone(),
            country_codes: item.request.country_codes.clone().ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "country_codes",
                },
            )?,
            language: item.request.language.clone().unwrap_or("en".to_string()),
            products: vec!["auth".to_string()],
            user: User {
                client_user_id: item.request.user_info.clone().ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "country_codes",
                    },
                )?,
            },
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct PlaidLinkTokenResponse {
    link_token: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, PlaidLinkTokenResponse, T, types::LinkTokenResponse>>
    for types::PaymentAuthRouterData<F, T, types::LinkTokenResponse>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PlaidLinkTokenResponse, T, types::LinkTokenResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::LinkTokenResponse {
                link_token: item.response.link_token,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct PlaidExchangeTokenRequest {
    public_token: String,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]

pub struct PlaidExchangeTokenResponse {
    pub access_token: String,
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<F, PlaidExchangeTokenResponse, T, types::ExchangeTokenResponse>,
    > for types::PaymentAuthRouterData<F, T, types::ExchangeTokenResponse>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            PlaidExchangeTokenResponse,
            T,
            types::ExchangeTokenResponse,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::ExchangeTokenResponse {
                access_token: item.response.access_token,
            }),
            ..item.data
        })
    }
}

impl TryFrom<&types::ExchangeTokenRouterData> for PlaidExchangeTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::ExchangeTokenRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            public_token: item.request.public_token.clone(),
        })
    }
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct PlaidRecipientCreateRequest {
    pub name: String,
    #[serde(flatten)]
    pub account_data: PlaidRecipientAccountData,
    pub address: Option<PlaidRecipientCreateAddress>,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct PlaidRecipientCreateResponse {
    pub recipient_id: String,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PlaidRecipientAccountData {
    Iban(Secret<String>),
    Bacs {
        sort_code: Secret<String>,
        account: Secret<String>,
    },
}

impl From<&types::RecipientAccountData> for PlaidRecipientAccountData {
    fn from(item: &types::RecipientAccountData) -> Self {
        match item {
            types::RecipientAccountData::Iban(iban) => Self::Iban(iban.clone()),
            types::RecipientAccountData::Bacs {
                sort_code,
                account_number,
            } => Self::Bacs {
                sort_code: sort_code.clone(),
                account: account_number.clone(),
            },
        }
    }
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct PlaidRecipientCreateAddress {
    pub street: String,
    pub city: String,
    pub postal_code: String,
    pub country: String,
}

impl From<&types::RecipientCreateAddress> for PlaidRecipientCreateAddress {
    fn from(item: &types::RecipientCreateAddress) -> Self {
        Self {
            street: item.street.clone(),
            city: item.city.clone(),
            postal_code: item.postal_code.clone(),
            country: common_enums::CountryAlpha2::to_string(&item.country),
        }
    }
}

impl TryFrom<&types::RecipientCreateRouterData> for PlaidRecipientCreateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RecipientCreateRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            name: item.request.name.clone(),
            account_data: PlaidRecipientAccountData::from(&item.request.account_data),
            address: item
                .request
                .address
                .as_ref()
                .map(PlaidRecipientCreateAddress::from),
        })
    }
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<
            F,
            PlaidRecipientCreateResponse,
            T,
            types::RecipientCreateResponse,
        >,
    > for types::PaymentAuthRouterData<F, T, types::RecipientCreateResponse>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            PlaidRecipientCreateResponse,
            T,
            types::RecipientCreateResponse,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RecipientCreateResponse {
                recipient_id: item.response.recipient_id,
            }),
            ..item.data
        })
    }
}
#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct PlaidBankAccountCredentialsRequest {
    access_token: String,
    options: Option<BankAccountCredentialsOptions>,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]

pub struct PlaidBankAccountCredentialsResponse {
    pub accounts: Vec<PlaidBankAccountCredentialsAccounts>,
    pub numbers: PlaidBankAccountCredentialsNumbers,
    // pub item: PlaidBankAccountCredentialsItem,
    pub request_id: String,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct BankAccountCredentialsOptions {
    account_ids: Vec<String>,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]

pub struct PlaidBankAccountCredentialsAccounts {
    pub account_id: String,
    pub name: String,
    pub subtype: Option<String>,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct PlaidBankAccountCredentialsBalances {
    pub available: Option<i32>,
    pub current: Option<i32>,
    pub limit: Option<i32>,
    pub iso_currency_code: Option<String>,
    pub unofficial_currency_code: Option<String>,
    pub last_updated_datetime: Option<String>,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct PlaidBankAccountCredentialsNumbers {
    pub ach: Vec<PlaidBankAccountCredentialsACH>,
    pub eft: Vec<PlaidBankAccountCredentialsEFT>,
    pub international: Vec<PlaidBankAccountCredentialsInternational>,
    pub bacs: Vec<PlaidBankAccountCredentialsBacs>,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct PlaidBankAccountCredentialsItem {
    pub item_id: String,
    pub institution_id: Option<String>,
    pub webhook: Option<String>,
    pub error: Option<PlaidErrorResponse>,
}
#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct PlaidBankAccountCredentialsACH {
    pub account_id: String,
    pub account: String,
    pub routing: String,
    pub wire_routing: Option<String>,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct PlaidBankAccountCredentialsEFT {
    pub account_id: String,
    pub account: String,
    pub institution: String,
    pub branch: String,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct PlaidBankAccountCredentialsInternational {
    pub account_id: String,
    pub iban: String,
    pub bic: String,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct PlaidBankAccountCredentialsBacs {
    pub account_id: String,
    pub account: String,
    pub sort_code: String,
}

impl TryFrom<&types::BankDetailsRouterData> for PlaidBankAccountCredentialsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::BankDetailsRouterData) -> Result<Self, Self::Error> {
        let options = item.request.optional_ids.as_ref().map(|bank_account_ids| {
            let ids = bank_account_ids
                .ids
                .iter()
                .map(|id| id.peek().to_string())
                .collect::<Vec<_>>();

            BankAccountCredentialsOptions { account_ids: ids }
        });

        Ok(Self {
            access_token: item.request.access_token.peek().to_string(),
            options,
        })
    }
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<
            F,
            PlaidBankAccountCredentialsResponse,
            T,
            types::BankAccountCredentialsResponse,
        >,
    > for types::PaymentAuthRouterData<F, T, types::BankAccountCredentialsResponse>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            PlaidBankAccountCredentialsResponse,
            T,
            types::BankAccountCredentialsResponse,
        >,
    ) -> Result<Self, Self::Error> {
        let (account_numbers, accounts_info) = (item.response.numbers, item.response.accounts);
        let mut bank_account_vec = Vec::new();
        let mut id_to_subtype = HashMap::new();

        accounts_info.into_iter().for_each(|acc| {
            id_to_subtype.insert(acc.account_id, (acc.subtype, acc.name));
        });

        account_numbers.ach.into_iter().for_each(|ach| {
            let (acc_type, acc_name) =
                if let Some((_type, name)) = id_to_subtype.get(&ach.account_id) {
                    (_type.to_owned(), Some(name.clone()))
                } else {
                    (None, None)
                };

            let account_details =
                types::PaymentMethodTypeDetails::Ach(types::BankAccountDetailsAch {
                    account_number: Secret::new(ach.account),
                    routing_number: Secret::new(ach.routing),
                });

            let bank_details_new = types::BankAccountDetails {
                account_name: acc_name,
                account_details,
                payment_method_type: PaymentMethodType::Ach,
                payment_method: PaymentMethod::BankDebit,
                account_id: ach.account_id.into(),
                account_type: acc_type,
            };

            bank_account_vec.push(bank_details_new);
        });

        account_numbers.bacs.into_iter().for_each(|bacs| {
            let (acc_type, acc_name) =
                if let Some((_type, name)) = id_to_subtype.get(&bacs.account_id) {
                    (_type.to_owned(), Some(name.clone()))
                } else {
                    (None, None)
                };

            let account_details =
                types::PaymentMethodTypeDetails::Bacs(types::BankAccountDetailsBacs {
                    account_number: Secret::new(bacs.account),
                    sort_code: Secret::new(bacs.sort_code),
                });

            let bank_details_new = types::BankAccountDetails {
                account_name: acc_name,
                account_details,
                payment_method_type: PaymentMethodType::Bacs,
                payment_method: PaymentMethod::BankDebit,
                account_id: bacs.account_id.into(),
                account_type: acc_type,
            };

            bank_account_vec.push(bank_details_new);
        });

        account_numbers.international.into_iter().for_each(|sepa| {
            let (acc_type, acc_name) =
                if let Some((_type, name)) = id_to_subtype.get(&sepa.account_id) {
                    (_type.to_owned(), Some(name.clone()))
                } else {
                    (None, None)
                };

            let account_details =
                types::PaymentMethodTypeDetails::Sepa(types::BankAccountDetailsSepa {
                    iban: Secret::new(sepa.iban),
                    bic: Secret::new(sepa.bic),
                });

            let bank_details_new = types::BankAccountDetails {
                account_name: acc_name,
                account_details,
                payment_method_type: PaymentMethodType::Sepa,
                payment_method: PaymentMethod::BankDebit,
                account_id: sepa.account_id.into(),
                account_type: acc_type,
            };

            bank_account_vec.push(bank_details_new);
        });

        Ok(Self {
            response: Ok(types::BankAccountCredentialsResponse {
                credentials: bank_account_vec,
            }),
            ..item.data
        })
    }
}
pub struct PlaidAuthType {
    pub client_id: Secret<String>,
    pub secret: Secret<String>,
    pub merchant_data: Option<types::MerchantRecipientData>,
}

impl TryFrom<&types::ConnectorAuthType> for PlaidAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { client_id, secret } => Ok(Self {
                client_id: client_id.to_owned(),
                secret: secret.to_owned(),
                merchant_data: None,
            }),
            types::ConnectorAuthType::OpenBankingAuth {
                api_key,
                key1,
                merchant_data,
            } => Ok(Self {
                client_id: api_key.to_owned(),
                secret: key1.to_owned(),
                merchant_data: Some(merchant_data.clone()),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct PlaidErrorResponse {
    pub display_message: Option<String>,
    pub error_code: Option<String>,
    pub error_message: String,
    pub error_type: Option<String>,
}
