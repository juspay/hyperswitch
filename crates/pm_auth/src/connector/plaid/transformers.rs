use std::collections::HashMap;

use common_enums::PaymentMethodType;
use masking::Secret;
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
        /// Tries to create a new instance of the current type from a LinkTokenRouterData item.
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
        /// Attempts to convert the provided ResponseRouterData into a Result containing an instance of Self.
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
        /// Attempts to convert the given `ResponseRouterData` into an instance of the current type.
    /// If successful, returns `Ok` with a new instance containing the `access_token` from the `ExchangeTokenResponse` in the `item` parameter. Otherwise, returns `Err` with the appropriate error.
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
        /// Attempts to create a new instance of the current type from the provided ExchangeTokenRouterData.
    /// 
    /// # Arguments
    /// 
    /// * `item` - A reference to the ExchangeTokenRouterData from which to create the new instance.
    /// 
    /// # Returns
    /// 
    /// Returns a Result containing either the new instance of the current type, or an error if the creation fails.
    /// 
    fn try_from(item: &types::ExchangeTokenRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            public_token: item.request.public_token.clone(),
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
        /// Attempts to create a new instance of Self from the given BankDetailsRouterData.
    /// 
    /// # Arguments
    /// 
    /// * `item` - A reference to the BankDetailsRouterData from which to create a new instance of Self.
    /// 
    /// # Returns
    /// 
    /// * `Result<Self, Self::Error>` - A Result containing the newly created instance of Self on success, or an error on failure.
    /// 
    fn try_from(item: &types::BankDetailsRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            access_token: item.request.access_token.clone(),
            options: item.request.optional_ids.as_ref().map(|bank_account_ids| {
                BankAccountCredentialsOptions {
                    account_ids: bank_account_ids.ids.clone(),
                }
            }),
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
        /// Attempts to convert the given ResponseRouterData into a Result of Self or Self::Error. 
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
        let mut id_to_suptype = HashMap::new();

        accounts_info.into_iter().for_each(|acc| {
            id_to_suptype.insert(acc.account_id, (acc.subtype, acc.name));
        });

        account_numbers.ach.into_iter().for_each(|ach| {
            let (acc_type, acc_name) =
                if let Some((_type, name)) = id_to_suptype.get(&ach.account_id) {
                    (_type.to_owned(), Some(name.clone()))
                } else {
                    (None, None)
                };

            let bank_details_new = types::BankAccountDetails {
                account_name: acc_name,
                account_number: ach.account,
                routing_number: ach.routing,
                payment_method_type: PaymentMethodType::Ach,
                account_id: ach.account_id,
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
}

impl TryFrom<&types::ConnectorAuthType> for PlaidAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
        /// Attempts to create a new instance of the ConnectorAuthType struct from the given types::ConnectorAuthType enum.
    /// 
    /// # Arguments
    /// 
    /// * `auth_type` - A reference to a types::ConnectorAuthType enum
    /// 
    /// # Returns
    /// 
    /// * `Result<Self, Self::Error>` - If the auth_type is of type BodyKey, returns Ok with a new instance of ConnectorAuthType populated with the client_id and secret. If the auth_type is not of type BodyKey, returns an error of type ConnectorError.
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { client_id, secret } => Ok(Self {
                client_id: client_id.to_owned(),
                secret: secret.to_owned(),
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
