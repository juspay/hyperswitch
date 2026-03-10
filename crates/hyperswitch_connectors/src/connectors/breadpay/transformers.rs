use common_enums::enums;
use common_utils::{request::Method, types::StringMinorUnit};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self, PaymentsAuthorizeRequestData},
};

pub struct BreadpayRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for BreadpayRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BreadpayCartRequest {
    custom_total: StringMinorUnit,
    options: Option<BreadpayCartOptions>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BreadpayCartOptions {
    order_ref: Option<String>,
    complete_url: String,
    callback_url: String,
    // billing_contact: Option<BillingContact>,
}

// #[derive(Debug, Serialize)]
// #[serde(rename_all = "camelCase")]
// pub struct BillingContact {
//     first_name: Secret<String>,
//     last_name: Secret<String>,
//     email: Option<Email>,
//     address: Secret<String>,
//     city: Secret<String>,
//     state: Secret<String>,
// }

impl TryFrom<&BreadpayRouterData<&PaymentsAuthorizeRouterData>> for BreadpayCartRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &BreadpayRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let request = match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::PayLater(pay_later_data) => match pay_later_data{
                hyperswitch_domain_models::payment_method_data::PayLaterData::BreadpayRedirect {  } => {
                                // let billing_contact = BillingContact {
                                //     first_name: item.router_data.get_billing_first_name()?,
                                //     last_name: item.router_data.get_billing_last_name()?,
                                //     email: item.router_data.get_optional_billing_email(),
                                //     address: item.router_data.get_billing_line1()?,
                                //     city: item.router_data.get_billing_city()?.into(),
                                //     state: item.router_data.get_billing_state()?,
                                // };
                                let options = Some({
                                    BreadpayCartOptions {
                                        order_ref: item.router_data.request.merchant_order_reference_id.clone(),
                                        complete_url: item.router_data.request.get_complete_authorize_url()?,
                                        callback_url: item.router_data.request.get_router_return_url()?
                                        // billing_contact: Some(billing_contact)
                                    }
                                });
                                Self{
                                    custom_total: item.amount.clone(),
                                    options,
                                }
                            },
                hyperswitch_domain_models::payment_method_data::PayLaterData::KlarnaRedirect {  } |
                hyperswitch_domain_models::payment_method_data::PayLaterData::WalleyRedirect {  } |
                            hyperswitch_domain_models::payment_method_data::PayLaterData::KlarnaSdk { .. } |
                            hyperswitch_domain_models::payment_method_data::PayLaterData::AffirmRedirect {  } |
                            hyperswitch_domain_models::payment_method_data::PayLaterData::FlexitiRedirect {  } |
                            hyperswitch_domain_models::payment_method_data::PayLaterData::AfterpayClearpayRedirect {  } |
                            hyperswitch_domain_models::payment_method_data::PayLaterData::PayBrightRedirect {  } |
                            hyperswitch_domain_models::payment_method_data::PayLaterData::AlmaRedirect {  } |
                            hyperswitch_domain_models::payment_method_data::PayLaterData::AtomeRedirect {  } |
                            hyperswitch_domain_models::payment_method_data::PayLaterData::PayjustnowRedirect {  } => {
                                Err(errors::ConnectorError::NotImplemented(
                                utils::get_unimplemented_payment_method_error_message("breadpay"),
                            ))
                            }?,
            },
            PaymentMethodData::Card(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(
                _,
            )
            | PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::Wallet(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::MobilePayment(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("breadpay"),
                ))
            }?
        };
        Ok(request)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BreadpayTransactionRequest {
    #[serde(rename = "type")]
    pub transaction_type: BreadpayTransactionType,
}

#[derive(Debug, Serialize)]
pub enum BreadpayTransactionType {
    Authorize,
    Settle,
    Cancel,
    Refund,
}

// Auth Struct
pub struct BreadpayAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) api_secret: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for BreadpayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_owned(),
                api_secret: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BreadpayTransactionResponse {
    status: TransactionStatus,
    bread_transactin_id: String,
    merchant_order_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionStatus {
    Pending,
    Canceled,
    Refunded,
    Expired,
    Authorized,
    Settled,
}

impl<F, T> TryFrom<ResponseRouterData<F, BreadpayTransactionResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, BreadpayTransactionResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status.clone()),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.bread_transactin_id),
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
}

impl From<TransactionStatus> for enums::AttemptStatus {
    fn from(item: TransactionStatus) -> Self {
        match item {
            TransactionStatus::Pending => Self::Pending,
            TransactionStatus::Authorized => Self::Authorized,
            TransactionStatus::Canceled => Self::Voided,
            TransactionStatus::Refunded => Self::AutoRefunded,
            TransactionStatus::Expired => Self::Failure,
            TransactionStatus::Settled => Self::Charged,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BreadpayPaymentsResponse {
    url: url::Url,
}

impl<F, T> TryFrom<ResponseRouterData<F, BreadpayPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, BreadpayPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            // As per documentation, the first call is cart creation where we don't get any status only get the customer redirection url.
            status: common_enums::AttemptStatus::AuthenticationPending,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::NoResponseId,
                redirection_data: Box::new(Some(RedirectForm::from((
                    item.response.url,
                    Method::Get,
                )))),
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
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CallBackResponse {
    pub transaction_id: String,
    pub order_ref: String,
}

// REFUND :
// Type definition for RefundRequest
#[derive(Debug, Serialize)]
pub struct BreadpayRefundRequest {
    pub amount: StringMinorUnit,
    #[serde(rename = "type")]
    pub transaction_type: BreadpayTransactionType,
}

impl<F> TryFrom<&BreadpayRouterData<&RefundsRouterData<F>>> for BreadpayRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &BreadpayRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
            transaction_type: BreadpayTransactionType::Refund,
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Copy, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
            //TODO: Review mapping
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreadpayErrorResponse {
    /// Human-readable error description
    pub description: String,

    /// Error type classification
    #[serde(rename = "type")]
    pub error_type: String,
}
