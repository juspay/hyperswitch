#[cfg(feature = "payouts")]
use api_models::payouts::{Bank, PayoutMethodData};
use common_enums::Currency;
#[cfg(feature = "payouts")]
use common_utils::pii;
#[cfg(feature = "payouts")]
use diesel_models::enums as storage_enums;
#[cfg(feature = "payouts")]
use error_stack::ResultExt;
#[cfg(feature = "payouts")]
use masking::PeekInterface;
use masking::Secret;
use serde::{Deserialize, Serialize};

#[cfg(feature = "payouts")]
use crate::connector::utils::{AddressDetailsData, RouterData};
#[cfg(feature = "payouts")]
use crate::types::api;
use crate::{
    connector::utils,
    core::errors,
    types::{self, api::CurrencyUnit},
};

pub struct WellsfargopayoutRouterData<T> {
    pub amount: String,
    pub router_data: T,
}

impl<T> TryFrom<(&CurrencyUnit, Currency, i64, T)> for WellsfargopayoutRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (_currency_unit, currency, amount, item): (&CurrencyUnit, Currency, i64, T),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: utils::to_currency_base_unit(amount, currency)?,
            router_data: item,
        })
    }
}
#[derive(Debug, Serialize)]
pub struct WellsfargopayoutAuthpdateRequest {
    grant_type: String,
    scope: String,
}
impl TryFrom<&types::RefreshTokenRouterData> for WellsfargopayoutAuthpdateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &types::RefreshTokenRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            grant_type: "client_credentials".to_string(),
            scope: "ACH-ALL".to_string(),
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WellsfargopayoutAuthUpdateResponse {
    pub access_token: Secret<String>,
    pub expires_in: i64,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, WellsfargopayoutAuthUpdateResponse, T, types::AccessToken>>
    for types::RouterData<F, T, types::AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            WellsfargopayoutAuthUpdateResponse,
            T,
            types::AccessToken,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::AccessToken {
                token: item.response.access_token,
                expires: item.response.expires_in,
            }),
            ..item.data
        })
    }
}

#[derive(Deserialize, Debug, Serialize)]
pub struct AccessTokenErrorResponse {
    pub error: String,
    pub error_description: String,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Clone)]
pub enum SecCode {
    PPD,
    CCD,
    WEB,
}
#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Debtor {
    ach_company_id: String,
}
#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct CreditTransfer {
    payment: PaymentInfo,
    creditor: Creditor,
}
#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentInfo {
    amount: String,
    currency: Currency,
}
#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Creditor {
    name: Secret<String>,
    bank_account: BankAccount,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankAccount {
    account_type: WellsfargopayoutPayoutAccountType,
    account_number: Secret<String>,
    routing_number: Secret<String>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WellsfargopayoutPayoutAccountType {
    CHECKING,
    SAVINGS,
    LOAN,
    GL,
}
#[cfg(feature = "payouts")]
impl From<storage_enums::BankType> for WellsfargopayoutPayoutAccountType {
    fn from(item: storage_enums::BankType) -> Self {
        match item {
            storage_enums::BankType::Checking => Self::CHECKING,
            storage_enums::BankType::Savings => Self::SAVINGS,
            storage_enums::BankType::Loan => Self::LOAN,
            storage_enums::BankType::GeneralLedger => Self::GL,
        }
    }
}

#[cfg(feature = "payouts")]
impl TryFrom<storage_enums::SecCode> for SecCode {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: storage_enums::SecCode) -> Result<Self, Self::Error> {
        match item {
            storage_enums::SecCode::PPD => Ok(Self::PPD),
            storage_enums::SecCode::CCD => Ok(Self::CCD),
            storage_enums::SecCode::WEB => Ok(Self::WEB),
            _ => Err(errors::ConnectorError::NotSupported {
                message: "Only PPD and CCD SecCode Allowed ".to_string(),
                connector: "Wellsfargopayout",
            }
            .into()),
        }
    }
}

#[cfg(feature = "payouts")]
#[derive(Debug, Deserialize)]
pub struct WellsfargopayoutPayoutMeta {
    ach_company_id: Secret<String>,
}
#[cfg(feature = "payouts")]
impl TryFrom<&Option<pii::SecretSerdeValue>> for WellsfargopayoutPayoutMeta {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: &Option<pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata",
            })?;
        Ok(metadata)
    }
}
#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Clone)]
pub struct WellsfargopayoutPayoutCreateRequest {
    sec_code: SecCode,
    debtor: Debtor,
    credit_transfer: CreditTransfer,
}

#[cfg(feature = "payouts")]
impl TryFrom<&WellsfargopayoutRouterData<&types::PayoutsRouterData<api::PoFulfill>>>
    for WellsfargopayoutPayoutCreateRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &WellsfargopayoutRouterData<&types::PayoutsRouterData<api::PoFulfill>>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.get_payout_method_data()? {
            PayoutMethodData::Bank(Bank::Ach(ach_data)) => {
                let metadata: WellsfargopayoutPayoutMeta = utils::to_connector_meta_from_secret(
                    item.router_data.connector_meta_data.clone(),
                )
                .change_context(
                    errors::ConnectorError::InvalidConnectorConfig { config: "metadata" },
                )?;
                let billing_address = item.router_data.get_billing_address()?;
                Ok(Self {
                    sec_code: SecCode::try_from(item.router_data.request.sec_code.ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "sec_code",
                        },
                    )?)?,
                    debtor: Debtor {
                        ach_company_id: metadata.ach_company_id.peek().to_owned(),
                    },
                    credit_transfer: CreditTransfer {
                        payment: PaymentInfo {
                            amount: item.amount.clone().to_string(),
                            currency: item.router_data.request.source_currency,
                        },
                        creditor: Creditor {
                            name: billing_address.get_full_name()?,
                            bank_account: BankAccount {
                                account_type: WellsfargopayoutPayoutAccountType::from(
                                    item.router_data.request.bank_type.ok_or(
                                        errors::ConnectorError::MissingRequiredField {
                                            field_name: "bank_type",
                                        },
                                    )?,
                                ),
                                account_number: ach_data.bank_account_number.clone(),
                                routing_number: ach_data.bank_routing_number.clone(),
                            },
                        },
                    },
                })
            }
            PayoutMethodData::Card(_) | PayoutMethodData::Bank(_) | PayoutMethodData::Wallet(_) => {
                Err(errors::ConnectorError::NotSupported {
                    message: "Payment Method Not Supported".to_string(),
                    connector: "Wellsfargopayout",
                })?
            }
        }
    }
}
#[allow(dead_code)]
pub struct WellsfargopayoutAuthType {
    pub(super) consumer_key: Secret<String>,
    pub(super) consumer_secret: Secret<String>,
    pub(super) gateway_entity_id: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for WellsfargopayoutAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::SignatureKey {
            api_key,
            api_secret,
            key1,
        } = auth_type
        {
            Ok(Self {
                consumer_key: api_key.to_owned(),
                consumer_secret: api_secret.to_owned(),
                gateway_entity_id: key1.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WellsfargopayoutPayoutStatus {
    PROCESSING,
    SUBMITTED,
    RECEIVED,
    SCHEDULED,
    SENT,
    REJECTED,
}

#[cfg(feature = "payouts")]
impl From<WellsfargopayoutPayoutStatus> for storage_enums::PayoutStatus {
    fn from(item: WellsfargopayoutPayoutStatus) -> Self {
        match item {
            WellsfargopayoutPayoutStatus::SENT => Self::Success,
            WellsfargopayoutPayoutStatus::SUBMITTED => Self::Initiated,
            WellsfargopayoutPayoutStatus::PROCESSING
            | WellsfargopayoutPayoutStatus::RECEIVED
            | WellsfargopayoutPayoutStatus::SCHEDULED => Self::Pending,
            WellsfargopayoutPayoutStatus::REJECTED => Self::Cancelled,
        }
    }
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WellsfargopayoutPayoutResponse {
    payment_id: String,
    status: WellsfargopayoutPayoutStatus,
}

#[cfg(feature = "payouts")]
impl<F> TryFrom<types::PayoutsResponseRouterData<F, WellsfargopayoutPayoutResponse>>
    for types::PayoutsRouterData<F>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PayoutsResponseRouterData<F, WellsfargopayoutPayoutResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::PayoutsResponseData {
                status: Some(storage_enums::PayoutStatus::from(item.response.status)),
                connector_payout_id: Some(item.response.payment_id),
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
                error_code: None,
                error_message: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WellsfargopayoutErrorResponse {
    pub errors: Vec<WellsfargopayoutError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WellsfargopayoutError {
    pub error_code: String,
    pub description: String,
}
#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
pub struct WellsFargoPayoutSyncResponse {
    payment_id: String,
    status: WellsfargopayoutPayoutStatus,
}

#[cfg(feature = "payouts")]
impl<F> TryFrom<types::PayoutsResponseRouterData<F, WellsFargoPayoutSyncResponse>>
    for types::PayoutsRouterData<F>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PayoutsResponseRouterData<F, WellsFargoPayoutSyncResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::PayoutsResponseData {
                status: Some(storage_enums::PayoutStatus::from(item.response.status)),
                connector_payout_id: Some(item.response.payment_id),
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
                error_code: None,
                error_message: None,
            }),
            ..item.data
        })
    }
}
