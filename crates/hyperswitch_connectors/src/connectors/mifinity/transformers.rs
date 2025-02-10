use common_enums::enums;
use common_utils::{
    pii::{self, Email},
    types::StringMajorUnit,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{PaymentMethodData, WalletData},
    router_data::{ConnectorAuthType, RouterData},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RedirectForm},
    types,
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::Date;

use crate::{
    types::ResponseRouterData,
    utils::{self, PaymentsAuthorizeRequestData, PhoneDetailsData, RouterData as _},
};

pub struct MifinityRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for MifinityRouterData<T> {
    fn from((amount, router_data): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data,
        }
    }
}
pub mod auth_headers {
    pub const API_VERSION: &str = "api-version";
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MifinityConnectorMetadataObject {
    pub brand_id: Secret<String>,
    pub destination_account_number: Secret<String>,
}

impl TryFrom<&Option<pii::SecretSerdeValue>> for MifinityConnectorMetadataObject {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: &Option<pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "merchant_connector_account.metadata",
            })?;
        Ok(metadata)
    }
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MifinityPaymentsRequest {
    money: Money,
    client: MifinityClient,
    address: MifinityAddress,
    validation_key: String,
    client_reference: common_utils::id_type::CustomerId,
    trace_id: String,
    description: String,
    destination_account_number: Secret<String>,
    brand_id: Secret<String>,
    return_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    language_preference: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Money {
    amount: StringMajorUnit,
    currency: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MifinityClient {
    first_name: Secret<String>,
    last_name: Secret<String>,
    phone: Secret<String>,
    dialing_code: String,
    nationality: api_models::enums::CountryAlpha2,
    email_address: Email,
    dob: Secret<Date>,
}

#[derive(Default, Debug, Serialize, PartialEq)]
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
        let metadata: MifinityConnectorMetadataObject =
            utils::to_connector_meta_from_secret(item.router_data.connector_meta_data.clone())
                .change_context(errors::ConnectorError::InvalidConnectorConfig {
                    config: "merchant_connector_account.metadata",
                })?;
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Wallet(wallet_data) => match wallet_data {
                WalletData::Mifinity(data) => {
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
                        dob: data.date_of_birth.clone(),
                    };
                    let address = MifinityAddress {
                        address_line1: item.router_data.get_billing_line1()?,
                        country_code: item.router_data.get_billing_country()?,
                        city: item.router_data.get_billing_city()?,
                    };
                    let validation_key = format!(
                        "payment_validation_key_{}_{}",
                        item.router_data.merchant_id.get_string_repr(),
                        item.router_data.connector_request_reference_id.clone()
                    );
                    let client_reference = item.router_data.customer_id.clone().ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "client_reference",
                        },
                    )?;
                    let destination_account_number = metadata.destination_account_number;
                    let trace_id = item.router_data.connector_request_reference_id.clone();
                    let brand_id = metadata.brand_id;
                    let language_preference = data.language_preference;
                    Ok(Self {
                        money,
                        client,
                        address,
                        validation_key,
                        client_reference,
                        trace_id: trace_id.clone(),
                        description: trace_id.clone(), //Connector recommend to use the traceId for a better experience in the BackOffice application later.
                        destination_account_number,
                        brand_id,
                        return_url: item.router_data.request.get_router_return_url()?,
                        language_preference,
                    })
                }
                WalletData::AliPayQr(_)
                | WalletData::AliPayRedirect(_)
                | WalletData::AliPayHkRedirect(_)
                | WalletData::AmazonPayRedirect(_)
                | WalletData::MomoRedirect(_)
                | WalletData::KakaoPayRedirect(_)
                | WalletData::GoPayRedirect(_)
                | WalletData::GcashRedirect(_)
                | WalletData::ApplePay(_)
                | WalletData::ApplePayRedirect(_)
                | WalletData::ApplePayThirdPartySdk(_)
                | WalletData::DanaRedirect {}
                | WalletData::GooglePay(_)
                | WalletData::GooglePayRedirect(_)
                | WalletData::GooglePayThirdPartySdk(_)
                | WalletData::MbWayRedirect(_)
                | WalletData::MobilePayRedirect(_)
                | WalletData::PaypalRedirect(_)
                | WalletData::PaypalSdk(_)
                | WalletData::Paze(_)
                | WalletData::SamsungPay(_)
                | WalletData::TwintRedirect {}
                | WalletData::VippsRedirect {}
                | WalletData::TouchNGoRedirect(_)
                | WalletData::WeChatPayRedirect(_)
                | WalletData::WeChatPayQr(_)
                | WalletData::CashappQr(_)
                | WalletData::SwishQr(_) => Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Mifinity"),
                )
                .into()),
            },
            PaymentMethodData::Card(_)
            | PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::MobilePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Mifinity"),
                )
                .into())
            }
        }
    }
}

// Auth Struct
pub struct MifinityAuthType {
    pub(super) key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for MifinityAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MifinityPaymentsResponse {
    payload: Vec<MifinityPayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MifinityPayload {
    trace_id: String,
    initialization_token: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, MifinityPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, MifinityPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let payload = item.response.payload.first();
        match payload {
            Some(payload) => {
                let trace_id = payload.trace_id.clone();
                let initialization_token = payload.initialization_token.clone();
                Ok(Self {
                    status: enums::AttemptStatus::AuthenticationPending,
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(trace_id.clone()),
                        redirection_data: Box::new(Some(RedirectForm::Mifinity {
                            initialization_token,
                        })),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: Some(trace_id),
                        incremental_authorization_allowed: None,
                        charges: None,
                    }),
                    ..item.data
                })
            }
            None => Ok(Self {
                status: enums::AttemptStatus::AuthenticationPending,
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::NoResponseId,
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            }),
        }
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
    payment_response: Option<PaymentResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentResponse {
    trace_id: Option<String>,
    client_reference: Option<String>,
    validation_key: Option<String>,
    transaction_reference: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MifinityPaymentStatus {
    Successful,
    Pending,
    Failed,
    NotCompleted,
}

impl<F, T> TryFrom<ResponseRouterData<F, MifinityPsyncResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, MifinityPsyncResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let payload = item.response.payload.first();

        match payload {
            Some(payload) => {
                let status = payload.to_owned().status.clone();
                let payment_response = payload.payment_response.clone();

                match payment_response {
                    Some(payment_response) => {
                        let transaction_reference = payment_response.transaction_reference.clone();
                        Ok(Self {
                            status: enums::AttemptStatus::from(status),
                            response: Ok(PaymentsResponseData::TransactionResponse {
                                resource_id: ResponseId::ConnectorTransactionId(
                                    transaction_reference,
                                ),
                                redirection_data: Box::new(None),
                                mandate_reference: Box::new(None),
                                connector_metadata: None,
                                network_txn_id: None,
                                connector_response_reference_id: None,
                                incremental_authorization_allowed: None,
                                charges: None,
                            }),
                            ..item.data
                        })
                    }
                    None => Ok(Self {
                        status: enums::AttemptStatus::from(status),
                        response: Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::NoResponseId,
                            redirection_data: Box::new(None),
                            mandate_reference: Box::new(None),
                            connector_metadata: None,
                            network_txn_id: None,
                            connector_response_reference_id: None,
                            incremental_authorization_allowed: None,
                            charges: None,
                        }),
                        ..item.data
                    }),
                }
            }
            None => Ok(Self {
                status: item.data.status,
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::NoResponseId,
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            }),
        }
    }
}

impl From<MifinityPaymentStatus> for enums::AttemptStatus {
    fn from(item: MifinityPaymentStatus) -> Self {
        match item {
            MifinityPaymentStatus::Successful => Self::Charged,
            MifinityPaymentStatus::Failed => Self::Failure,
            MifinityPaymentStatus::NotCompleted => Self::AuthenticationPending,
            MifinityPaymentStatus::Pending => Self::Pending,
        }
    }
}

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
