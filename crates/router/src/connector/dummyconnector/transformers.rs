use diesel_models::enums::Currency;
use masking::Secret;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    connector::utils::RouterData,
    core::errors,
    services,
    types::{self, api, domain, storage::enums},
};

#[derive(Debug, Serialize, strum::Display, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum DummyConnectors {
    #[serde(rename = "phonypay")]
    #[strum(serialize = "phonypay")]
    PhonyPay,
    #[serde(rename = "fauxpay")]
    #[strum(serialize = "fauxpay")]
    FauxPay,
    #[serde(rename = "pretendpay")]
    #[strum(serialize = "pretendpay")]
    PretendPay,
    StripeTest,
    AdyenTest,
    CheckoutTest,
    PaypalTest,
}

impl DummyConnectors {
    pub fn get_dummy_connector_id(self) -> &'static str {
        match self {
            Self::PhonyPay => "phonypay",
            Self::FauxPay => "fauxpay",
            Self::PretendPay => "pretendpay",
            Self::StripeTest => "stripe_test",
            Self::AdyenTest => "adyen_test",
            Self::CheckoutTest => "checkout_test",
            Self::PaypalTest => "paypal_test",
        }
    }
}

impl From<u8> for DummyConnectors {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::PhonyPay,
            2 => Self::FauxPay,
            3 => Self::PretendPay,
            4 => Self::StripeTest,
            5 => Self::AdyenTest,
            6 => Self::CheckoutTest,
            7 => Self::PaypalTest,
            _ => Self::PhonyPay,
        }
    }
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct DummyConnectorPaymentsRequest<const T: u8> {
    amount: i64,
    currency: Currency,
    payment_method_data: PaymentMethodData,
    return_url: Option<String>,
    connector: DummyConnectors,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PaymentMethodData {
    Card(DummyConnectorCard),
    Wallet(DummyConnectorWallet),
    PayLater(DummyConnectorPayLater),
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct DummyConnectorCard {
    name: Secret<String>,
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
}

impl TryFrom<(domain::Card, Option<Secret<String>>)> for DummyConnectorCard {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (value, card_holder_name): (domain::Card, Option<Secret<String>>),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            name: card_holder_name.unwrap_or(Secret::new("".to_string())),
            number: value.card_number,
            expiry_month: value.card_exp_month,
            expiry_year: value.card_exp_year,
            cvc: value.card_cvc,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum DummyConnectorWallet {
    GooglePay,
    Paypal,
    WeChatPay,
    MbWay,
    AliPay,
    AliPayHK,
}

impl TryFrom<domain::WalletData> for DummyConnectorWallet {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: domain::WalletData) -> Result<Self, Self::Error> {
        match value {
            domain::WalletData::GooglePayRedirect(_) => Ok(Self::GooglePay),
            domain::WalletData::PaypalRedirect(_) => Ok(Self::Paypal),
            domain::WalletData::WeChatPayRedirect(_) => Ok(Self::WeChatPay),
            domain::WalletData::MbWayRedirect(_) => Ok(Self::MbWay),
            domain::WalletData::AliPayRedirect(_) => Ok(Self::AliPay),
            domain::WalletData::AliPayHkRedirect(_) => Ok(Self::AliPayHK),
            _ => Err(errors::ConnectorError::NotImplemented("Dummy wallet".to_string()).into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum DummyConnectorPayLater {
    Klarna,
    Affirm,
    AfterPayClearPay,
}

impl TryFrom<domain::payments::PayLaterData> for DummyConnectorPayLater {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: domain::payments::PayLaterData) -> Result<Self, Self::Error> {
        match value {
            domain::payments::PayLaterData::KlarnaRedirect { .. } => Ok(Self::Klarna),
            domain::payments::PayLaterData::AffirmRedirect {} => Ok(Self::Affirm),
            domain::payments::PayLaterData::AfterpayClearpayRedirect { .. } => {
                Ok(Self::AfterPayClearPay)
            }
            _ => Err(errors::ConnectorError::NotImplemented("Dummy pay later".to_string()).into()),
        }
    }
}

impl<const T: u8> TryFrom<&types::PaymentsAuthorizeRouterData>
    for DummyConnectorPaymentsRequest<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let payment_method_data: Result<PaymentMethodData, Self::Error> = match item
            .request
            .payment_method_data
        {
            domain::PaymentMethodData::Card(ref req_card) => {
                let card_holder_name = item.get_optional_billing_full_name();
                Ok(PaymentMethodData::Card(DummyConnectorCard::try_from((
                    req_card.clone(),
                    card_holder_name,
                ))?))
            }
            domain::PaymentMethodData::Wallet(ref wallet_data) => {
                Ok(PaymentMethodData::Wallet(wallet_data.clone().try_into()?))
            }
            domain::PaymentMethodData::PayLater(ref pay_later_data) => Ok(
                PaymentMethodData::PayLater(pay_later_data.clone().try_into()?),
            ),
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        };
        Ok(Self {
            amount: item.request.amount,
            currency: item.request.currency,
            payment_method_data: payment_method_data?,
            return_url: item.request.router_return_url.clone(),
            connector: Into::<DummyConnectors>::into(T),
        })
    }
}

// Auth Struct
pub struct DummyConnectorAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for DummyConnectorAuthType {
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
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DummyConnectorPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<DummyConnectorPaymentStatus> for enums::AttemptStatus {
    fn from(item: DummyConnectorPaymentStatus) -> Self {
        match item {
            DummyConnectorPaymentStatus::Succeeded => Self::Charged,
            DummyConnectorPaymentStatus::Failed => Self::Failure,
            DummyConnectorPaymentStatus::Processing => Self::AuthenticationPending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaymentsResponse {
    status: DummyConnectorPaymentStatus,
    id: String,
    amount: i64,
    currency: Currency,
    created: String,
    payment_method_type: PaymentMethodType,
    next_action: Option<DummyConnectorNextAction>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PaymentMethodType {
    Card,
    Wallet(DummyConnectorWallet),
    PayLater(DummyConnectorPayLater),
}

impl<F, T> TryFrom<types::ResponseRouterData<F, PaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirection_data = item
            .response
            .next_action
            .and_then(|redirection_data| redirection_data.get_url())
            .map(|redirection_url| {
                services::RedirectForm::from((redirection_url, services::Method::Get))
            });
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: Box::new(redirection_data),
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DummyConnectorNextAction {
    RedirectToUrl(Url),
}

impl DummyConnectorNextAction {
    fn get_url(&self) -> Option<Url> {
        match self {
            Self::RedirectToUrl(redirect_to_url) => Some(redirect_to_url.to_owned()),
        }
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct DummyConnectorRefundRequest {
    pub amount: i64,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for DummyConnectorRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.request.refund_amount,
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
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

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
    currency: Currency,
    created: String,
    payment_amount: i64,
    refund_amount: i64,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct DummyConnectorErrorResponse {
    pub error: ErrorData,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct ErrorData {
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
