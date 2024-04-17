#[cfg(feature = "payouts")]
use api_models::payouts::{Bank, PayoutMethodData};
use common_enums::Currency;
#[cfg(feature = "payouts")]
use common_utils::pii::Email;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};
 
#[cfg(feature = "payouts")]
use crate::{
    types::{
        api,
        storage::enums as storage_enums
    },
    connector::utils::{self, AddressDetailsData, RouterData}
};
use crate::{
    core::errors,
    types::{
        self,
        api::CurrencyUnit,
    },
};

pub struct EbanxRouterData<T> {
    pub amount: f64, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> TryFrom<(&CurrencyUnit, Currency, i64, T)> for EbanxRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (_currency_unit, currency, amount, item): (&CurrencyUnit, Currency, i64, T),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: utils::to_currency_base_unit_asf64(amount, currency)?,
            router_data: item,
        })
    }
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Clone)]
pub struct EbanxPayoutCreateRequest {
    integration_key: Secret<String>,
    external_reference: String,
    country: String,
    amount: f64,
    currency: Currency,
    target: EbanxPayoutType,
    target_account: Secret<String>,
    payee: EbanxPayoutDetails,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Clone)]
pub enum EbanxPayoutType {
    BankAccount,
    Mercadopago,
    EwalletNequi,
    PixKey,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Clone)]
pub struct EbanxPayoutDetails {
    name: Secret<String>,
    email: Option<Email>,
    document: Option<Secret<String>>,
    document_type: Option<EbanxDocumentType>,
    bank_info: EbanxBankDetails,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Clone)]
pub enum EbanxDocumentType {
    #[serde(rename = "CPF")]
    NaturalPersonsRegister,
    #[serde(rename = "CNPJ")]
    NationalRegistryOfLegalEntities,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Clone)]
pub struct EbanxBankDetails {
    bank_name: Option<String>,
    bank_branch: Option<String>,
    bank_account: Option<Secret<String>>,
    account_type: Option<EbanxBankAccountType>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Clone)]
pub enum EbanxBankAccountType {
    #[serde(rename = "C")]
    CheckingAccount,
}

#[cfg(feature = "payouts")]
impl TryFrom<&EbanxRouterData<&types::PayoutsRouterData<api::PoCreate>>>
    for EbanxPayoutCreateRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &EbanxRouterData<&types::PayoutsRouterData<api::PoCreate>>,
    ) -> Result<Self, Self::Error> {
        let ebanx_auth_type = EbanxAuthType::try_from(&item.router_data.connector_auth_type)?;
        match item.router_data.get_payout_method_data()? {
            PayoutMethodData::Bank(Bank::Pix(pix_data)) => {
                let bank_info = EbanxBankDetails {
                    bank_account: Some(pix_data.bank_account_number),
                    bank_branch: pix_data.bank_branch,
                    bank_name: pix_data.bank_name,
                    account_type: Some(EbanxBankAccountType::CheckingAccount),
                };

                let billing_address = item.router_data.get_billing_address()?;
                let customer_details = item.router_data.request.customer_details.clone().ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "Customer Details",
                    },
                )?;

                let document_type = if pix_data.tax_id.clone().expose().len() == 11 {
                    Some(EbanxDocumentType::NaturalPersonsRegister)
                } else {
                    Some(EbanxDocumentType::NationalRegistryOfLegalEntities)
                };

                let payee = EbanxPayoutDetails {
                    name: billing_address.get_full_name()?,
                    email: customer_details.email.clone(),
                    bank_info,
                    document_type,
                    document: Some(pix_data.tax_id.to_owned()),
                };
                Ok(Self {
                    amount: item.amount,
                    integration_key: ebanx_auth_type.integration_key,
                    country: customer_details.phone_country_code.ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "Customer Details",
                        },
                    )?,
                    currency: item.router_data.request.source_currency,
                    external_reference: item.router_data.connector_request_reference_id.to_owned(),
                    target: EbanxPayoutType::PixKey,
                    target_account: pix_data.pix_key,
                    payee,
                })
            }
            _ => Err(errors::ConnectorError::NotSupported {
                message: "Payment Method Not Supported".to_string(),
                connector: "Ebanx",
            })?,
        }
    }
}

pub struct EbanxAuthType {
    pub(super) integration_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for EbanxAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                integration_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EbanxPayoutStatus {
    #[serde(rename = "PA")]
    Succeeded,
    #[serde(rename = "CA")]
    Cancelled,
    #[serde(rename = "PE")]
    Processing,
    #[serde(rename = "OP")]
    RequiresFulfillment,
}

#[cfg(feature = "payouts")]
impl From<EbanxPayoutStatus> for storage_enums::PayoutStatus {
    fn from(item: EbanxPayoutStatus) -> Self {
        match item {
            EbanxPayoutStatus::Succeeded => Self::Success,
            EbanxPayoutStatus::Cancelled => Self::Cancelled,
            EbanxPayoutStatus::Processing => Self::Pending,
            EbanxPayoutStatus::RequiresFulfillment => Self::RequiresFulfillment,
        }
    }
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EbanxPayoutResponse {
    payout: EbanxPayoutResponseDetails,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EbanxPayoutResponseDetails {
    uid: String,
    status: EbanxPayoutStatus,
}

#[cfg(feature = "payouts")]
impl<F> TryFrom<types::PayoutsResponseRouterData<F, EbanxPayoutResponse>>
    for types::PayoutsRouterData<F>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PayoutsResponseRouterData<F, EbanxPayoutResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::PayoutsResponseData {
                status: Some(storage_enums::PayoutStatus::from(
                    item.response.payout.status,
                )),
                connector_payout_id: item.response.payout.uid,
                payout_eligible: None,
            }),
            ..item.data
        })
    }
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EbanxPayoutFulfillRequest {
    integration_key: Secret<String>,
    uid: String,
}

#[cfg(feature = "payouts")]
impl<F> TryFrom<&EbanxRouterData<&types::PayoutsRouterData<F>>> for EbanxPayoutFulfillRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &EbanxRouterData<&types::PayoutsRouterData<F>>) -> Result<Self, Self::Error> {
        let request = item.router_data.request.to_owned();
        let ebanx_auth_type = EbanxAuthType::try_from(&item.router_data.connector_auth_type)?;
        match request.payout_type.to_owned() {
            storage_enums::PayoutType::Bank => Ok(Self {
                integration_key: ebanx_auth_type.integration_key,
                uid: request
                    .connector_payout_id
                    .to_owned()
                    .ok_or(errors::ConnectorError::MissingRequiredField { field_name: "uid" })?,
            }),
            storage_enums::PayoutType::Card | storage_enums::PayoutType::Wallet => {
                Err(errors::ConnectorError::NotSupported {
                    message: "Payout Method Not Supported".to_string(),
                    connector: "Ebanx",
                })?
            }
        }
    }
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EbanxFulfillResponse {
    #[serde(rename = "type")]
    status: EbanxFulfillStatus,
    message: String,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EbanxFulfillStatus {
    Success,
    ApiError,
    AuthenticationError,
    InvalidRequestError,
    RequestError,
}

#[cfg(feature = "payouts")]
impl From<EbanxFulfillStatus> for storage_enums::PayoutStatus {
    fn from(item: EbanxFulfillStatus) -> Self {
        match item {
            EbanxFulfillStatus::Success => Self::Success,
            EbanxFulfillStatus::ApiError
            | EbanxFulfillStatus::AuthenticationError
            | EbanxFulfillStatus::InvalidRequestError
            | EbanxFulfillStatus::RequestError => Self::Failed,
        }
    }
}

#[cfg(feature = "payouts")]
impl<F> TryFrom<types::PayoutsResponseRouterData<F, EbanxFulfillResponse>>
    for types::PayoutsRouterData<F>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PayoutsResponseRouterData<F, EbanxFulfillResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::PayoutsResponseData {
                status: Some(storage_enums::PayoutStatus::from(item.response.status)),
                connector_payout_id: item
                    .data
                    .request
                    .connector_payout_id
                    .clone()
                    .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?,
                payout_eligible: None,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct EbanxErrorResponse {
    pub code: String,
    pub status_code: String,
    pub message: Option<String>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EbanxPayoutCancelRequest {
    integration_key: Secret<String>,
    uid: String,
}

#[cfg(feature = "payouts")]
impl<F> TryFrom<&types::PayoutsRouterData<F>> for EbanxPayoutCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PayoutsRouterData<F>) -> Result<Self, Self::Error> {
        let request = item.request.to_owned();
        let ebanx_auth_type = EbanxAuthType::try_from(&item.connector_auth_type)?;
        match request.payout_type.to_owned() {
            storage_enums::PayoutType::Bank => Ok(Self {
                integration_key: ebanx_auth_type.integration_key,
                uid: request
                    .connector_payout_id
                    .to_owned()
                    .ok_or(errors::ConnectorError::MissingRequiredField { field_name: "uid" })?,
            }),
            storage_enums::PayoutType::Card | storage_enums::PayoutType::Wallet => {
                Err(errors::ConnectorError::NotSupported {
                    message: "Payout Method Not Supported".to_string(),
                    connector: "Ebanx",
                })?
            }
        }
    }
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EbanxCancelResponse {
    #[serde(rename = "type")]
    status: EbanxCancelStatus,
    message: String,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EbanxCancelStatus {
    Success,
    ApiError,
    AuthenticationError,
    InvalidRequestError,
    RequestError,
}

#[cfg(feature = "payouts")]
impl From<EbanxCancelStatus> for storage_enums::PayoutStatus {
    fn from(item: EbanxCancelStatus) -> Self {
        match item {
            EbanxCancelStatus::Success => Self::Cancelled,
            EbanxCancelStatus::ApiError
            | EbanxCancelStatus::AuthenticationError
            | EbanxCancelStatus::InvalidRequestError
            | EbanxCancelStatus::RequestError => Self::Failed,
        }
    }
}

#[cfg(feature = "payouts")]
impl<F> TryFrom<types::PayoutsResponseRouterData<F, EbanxCancelResponse>>
    for types::PayoutsRouterData<F>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PayoutsResponseRouterData<F, EbanxCancelResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::PayoutsResponseData {
                status: Some(storage_enums::PayoutStatus::from(item.response.status)),
                connector_payout_id: item
                    .data
                    .request
                    .connector_payout_id
                    .clone()
                    .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?,
                payout_eligible: None,
            }),
            ..item.data
        })
    }
}
