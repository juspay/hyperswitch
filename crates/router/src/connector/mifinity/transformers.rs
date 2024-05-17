use common_utils::pii::{self, Email};
use error_stack::ResultExt;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, PhoneDetailsData, RouterData},
    core::errors::{self, CustomResult},
    services,
    types::{self, domain, storage::enums},
};

//TODO: Fill the struct with respective fields
pub struct MifinityRouterData<T> {
    pub amount: String,
    pub router_data: T,
}

impl<T> TryFrom<(&api::CurrencyUnit, enums::Currency, i64, T)> for MifinityRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (_currency_unit, _currency, amount, item): (&api::CurrencyUnit, enums::Currency, i64, T),
    ) -> Result<Self, Self::Error> {
        let amount = utils::get_amount_as_string(currency_unit, amount, currency)?;
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

pub mod auth_headers {
    pub const API_VERSION: &str = "api-version";
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MifinityConnectorMetadataObject {
    pub brand_id: Option<String>,
}

impl TryFrom<&Option<pii::SecretSerdeValue>> for MifinityConnectorMetadataObject {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: &Option<pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata",
            })?;
        Ok(metadata)
    }
}

fn get_brand_id_for_mifinity(
    connector_metadata: &Option<common_utils::pii::SecretSerdeValue>,
) -> CustomResult<String, errors::ConnectorError> {
    let mifinity_metadata = MifinityConnectorMetadataObject::try_from(connector_metadata)?;
    let brand_id =
        mifinity_metadata
            .brand_id
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "brand_id",
            })?;
    Ok(brand_id)
}

//TODO: Fill the struct with respective fields
#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MifinityPaymentsRequest {
    money: Money,
    client: MifinityClient,
    address: MifinityAddress,
    validation_key: String,
    client_reference: String,
    trace_id: Secret<String>,
    description: String,
    destination_account_number: Secret<String>,
    brand_id: Secret<String>,
    return_url: String,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct Money {
    amount: String,
    currency: String,
}

#[derive(Debug, Clone, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MifinityClient {
    first_name: Secret<String>,
    last_name: Secret<String>,
    phone: Secret<String>,
    dialing_code: String,
    nationality: api_models::enums::CountryAlpha2,
    email_address: Email,
    dob: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MifinityAddress {
    address_line1: Secret<String>,
    country_code: api_models::enums::CountryAlpha2,
    city: String,
}

impl TryFrom<&MifinityRouterData<&types::PaymentsAuthorizeRouterData>> for MifinityPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &MifinityRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            domain::PaymentMethodData::Wallet(wallet_data) => match wallet_data {
                domain::WalletData::Mifinity(data) => {
                    let money = Money {
                        amount: item.amount.clone(),
                        currency: item.router_data.request.currency.to_string(),
                    };
                    let phone_details = item.router_data.get_billing_phone()?;
                    let client = MifinityClient {
                        first_name: item.router_data.get_billing_first_name()?,
                        last_name: item.router_data.get_billing_last_name()?,
                        phone: phone_details.get_number()?,
                        dialing_code: phone_details.get_country_code()?,
                        nationality: item.router_data.get_billing_country()?,
                        email_address: item.router_data.get_billing_email()?,
                        dob: data.dob.clone(),
                    };
                    let address = MifinityAddress {
                        address_line1: item.router_data.get_billing_line1()?,
                        country_code: item.router_data.get_billing_country()?,
                        city: item.router_data.get_billing_city()?,
                    };
                    let validation_key = format!(
                        "payment_validation_key_{}_{}",
                        item.router_data.merchant_id,
                        item.router_data.connector_request_reference_id.clone()
                    );
                    let client_reference = item.router_data.customer_id.clone().ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "client_reference",
                        },
                    )?;
                    let destination_account_number = data.destination_account_number;
                    let trace_id = item.router_data.connector_request_reference_id.clone();
                    let brand_id = Secret::new(get_brand_id_for_mifinity(
                        &item.router_data.connector_meta_data,
                    )?);
                    Ok(Self {
                        money,
                        client,
                        address,
                        validation_key,
                        client_reference,
                        trace_id: Secret::new(trace_id.clone()),
                        description: trace_id.clone(),
                        destination_account_number,
                        brand_id,
                        return_url: item.router_data.return_url.clone().ok_or(
                            errors::ConnectorError::MissingRequiredField {
                                field_name: "return_url",
                            },
                        )?,
                    })
                }
                domain::WalletData::AliPayQr(_)
                | domain::WalletData::AliPayRedirect(_)
                | domain::WalletData::AliPayHkRedirect(_)
                | domain::WalletData::MomoRedirect(_)
                | domain::WalletData::KakaoPayRedirect(_)
                | domain::WalletData::GoPayRedirect(_)
                | domain::WalletData::GcashRedirect(_)
                | domain::WalletData::ApplePay(_)
                | domain::WalletData::ApplePayRedirect(_)
                | domain::WalletData::ApplePayThirdPartySdk(_)
                | domain::WalletData::DanaRedirect {}
                | domain::WalletData::GooglePay(_)
                | domain::WalletData::GooglePayRedirect(_)
                | domain::WalletData::GooglePayThirdPartySdk(_)
                | domain::WalletData::MbWayRedirect(_)
                | domain::WalletData::MobilePayRedirect(_)
                | domain::WalletData::PaypalRedirect(_)
                | domain::WalletData::PaypalSdk(_)
                | domain::WalletData::SamsungPay(_)
                | domain::WalletData::TwintRedirect {}
                | domain::WalletData::VippsRedirect {}
                | domain::WalletData::TouchNGoRedirect(_)
                | domain::WalletData::WeChatPayRedirect(_)
                | domain::WalletData::WeChatPayQr(_)
                | domain::WalletData::CashappQr(_)
                | domain::WalletData::SwishQr(_) => Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Mifinity"),
                )
                .into()),
            },
            domain::PaymentMethodData::Card(_)
            | domain::PaymentMethodData::CardRedirect(_)
            | domain::PaymentMethodData::BankRedirect(_)
            | domain::PaymentMethodData::PayLater(_)
            | domain::PaymentMethodData::BankDebit(_)
            | domain::PaymentMethodData::BankTransfer(_)
            | domain::PaymentMethodData::Crypto(_)
            | domain::PaymentMethodData::MandatePayment
            | domain::PaymentMethodData::Reward
            | domain::PaymentMethodData::Upi(_)
            | domain::PaymentMethodData::Voucher(_)
            | domain::PaymentMethodData::GiftCard(_)
            | domain::PaymentMethodData::CardToken(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Mifinity"),
                )
                .into())
            }
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct MifinityAuthType {
    pub(super) key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for MifinityAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MifinityPaymentsResponse {
    payload: Vec<MifinityPayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MifinityPayload {
    trace_id: String,
    initialization_token: String,
    client: MifinityClientResponse,
    address: MifinityAddressResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MifinityClientResponse {
    first_name: Secret<String>,
    last_name: Secret<String>,
    phone: Secret<String>,
    dialing_code: String,
    nationality: String,
    email_address: String,
    dob: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MifinityAddressResponse {
    address_line1: String,
    country_code: String,
    city: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, MifinityPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            MifinityPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let trace_id: String = item
            .response
            .payload
            .iter()
            .map(|payload| payload.trace_id.clone())
            .collect();
        let initialization_token = item
            .response
            .payload
            .iter()
            .map(|payload| payload.initialization_token.clone())
            .collect();
        Ok(Self {
            status: enums::AttemptStatus::AuthenticationPending,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::NoResponseId,
                redirection_data: Some(services::RedirectForm::Mifinity {
                    initialization_token,
                }),
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(trace_id),
                incremental_authorization_allowed: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MifinityPsyncResponse {
    payload: Vec<MifinityPsyncPayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MifinityPsyncPayload {
    status: MifinityPaymentStatus,
    payment_response: PaymentResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentResponse {
    trace_id: Option<String>,
    client_reference: Option<String>,
    validation_key: Option<String>,
    transaction_reference: String,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MifinityPaymentStatus {
    Successful,
    #[default]
    Pending,
    Failed,
    NotCompleted,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, MifinityPsyncResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, MifinityPsyncResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let transaction_reference = item
            .response
            .payload
            .iter()
            .map(|payload| payload.payment_response.transaction_reference.clone())
            .collect();

        Ok(Self {
            status: enums::AttemptStatus::Charged,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(transaction_reference),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct MifinityErrorResponse {
    pub errors: Vec<MifinityErrorList>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MifinityErrorList {
    #[serde(rename = "type")]
    pub error_type: String,
    pub error_code: String,
    pub message: String,
    pub field: Option<String>,
}
