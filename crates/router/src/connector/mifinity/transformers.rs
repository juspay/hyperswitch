use common_utils::pii::Email;
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::Date;

use crate::{
    connector::utils::{self, PhoneDetailsData, RouterData},
    core::errors,
    types::{self, domain, storage::enums},
};

//TODO: Fill the struct with respective fields
pub struct MifinityRouterData<T> {
    pub amount: String,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for MifinityRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (currency_unit, currency, amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        let amount = utils::get_amount_as_string(currency_unit, amount, currency)?;
        Ok(Self {
            amount,
            router_data: item,
        })
    }
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
    // dob: Date,
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
                        // dob: todo!(),
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
                    let client_reference = item.router_data.request.customer_id.clone().ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "client_reference",
                        },
                    )?;
                    let destination_account_number = data.destination_account_number;
                    let trace_id = item.router_data.connector_request_reference_id.clone();
                    Ok(Self {
                        money,
                        client,
                        address,
                        validation_key,
                        client_reference,
                        trace_id: Secret::new(trace_id.clone()),
                        description: trace_id.clone(),
                        destination_account_number,
                        brand_id: Secret::new("001".to_string()),
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
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for MifinityAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
// #[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
// #[serde(rename_all = "lowercase")]
// pub enum MifinityPaymentStatus {
//     Succeeded,
//     Failed,
//     #[default]
//     Processing,
// }

// impl From<MifinityPaymentStatus> for enums::AttemptStatus {
//     fn from(item: MifinityPaymentStatus) -> Self {
//         match item {
//             MifinityPaymentStatus::Succeeded => Self::Charged,
//             MifinityPaymentStatus::Failed => Self::Failure,
//             MifinityPaymentStatus::Processing => Self::Authorizing,
//         }
//     }
// }

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
    dob: Date,
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
        Ok(Self {
            status: enums::AttemptStatus::Pending,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response
                        .payload
                        .iter()
                        .map(|payload| payload.trace_id.clone())
                        .collect(),
                ),
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
// REFUND :
// Type definition for RefundRequest
// #[derive(Default, Debug, Serialize)]
// pub struct MifinityRefundRequest {
//     pub amount: String,
// }

// impl<F> TryFrom<&MifinityRouterData<&types::RefundsRouterData<F>>> for MifinityRefundRequest {
//     type Error = error_stack::Report<errors::ConnectorError>;
//     fn try_from(
//         item: &MifinityRouterData<&types::RefundsRouterData<F>>,
//     ) -> Result<Self, Self::Error> {
//         Ok(Self {
//             amount: item.amount.to_owned(),
//         })
//     }
// }

// // Type definition for Refund Response

// #[allow(dead_code)]
// #[derive(Debug, Serialize, Default, Deserialize, Clone)]
// pub enum RefundStatus {
//     Succeeded,
//     Failed,
//     #[default]
//     Processing,
// }

// impl From<RefundStatus> for enums::RefundStatus {
//     fn from(item: RefundStatus) -> Self {
//         match item {
//             RefundStatus::Succeeded => Self::Success,
//             RefundStatus::Failed => Self::Failure,
//             RefundStatus::Processing => Self::Pending,
//             //TODO: Review mapping
//         }
//     }
// }

// //TODO: Fill the struct with respective fields
// #[derive(Default, Debug, Clone, Serialize, Deserialize)]
// pub struct RefundResponse {
//     id: String,
//     status: RefundStatus,
// }

// impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
//     for types::RefundsRouterData<api::Execute>
// {
//     type Error = error_stack::Report<errors::ConnectorError>;
//     fn try_from(
//         item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
//     ) -> Result<Self, Self::Error> {
//         Ok(Self {
//             response: Ok(types::RefundsResponseData {
//                 connector_refund_id: item.response.id.to_string(),
//                 refund_status: enums::RefundStatus::from(item.response.status),
//             }),
//             ..item.data
//         })
//     }
// }

// impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
//     for types::RefundsRouterData<api::RSync>
// {
//     type Error = error_stack::Report<errors::ConnectorError>;
//     fn try_from(
//         item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
//     ) -> Result<Self, Self::Error> {
//         Ok(Self {
//             response: Ok(types::RefundsResponseData {
//                 connector_refund_id: item.response.id.to_string(),
//                 refund_status: enums::RefundStatus::from(item.response.status),
//             }),
//             ..item.data
//         })
//     }
// }

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct MifinityErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
