use common_enums::{AttemptStatus, Currency, RefundStatus};
use common_utils::{pii, request::Method, types::MinorUnit};
use hyperswitch_domain_models::{
    payment_method_data::{
        Card, PayLaterData, PaymentMethodData, UpiCollectData, UpiData, WalletData,
    },
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors::ConnectorError;
use masking::Secret;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::RouterData as _,
};

#[derive(Debug, Serialize)]
pub struct DummyConnectorRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for DummyConnectorRouterData<T> {
    fn from((amount, router_data): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data,
        }
    }
}

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
    amount: MinorUnit,
    currency: Currency,
    payment_method_data: DummyPaymentMethodData,
    return_url: Option<String>,
    connector: DummyConnectors,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DummyPaymentMethodData {
    Card(DummyConnectorCard),
    Wallet(DummyConnectorWallet),
    PayLater(DummyConnectorPayLater),
    Upi(DummyConnectorUpi),
}

#[derive(Clone, Debug, serde::Serialize, Eq, PartialEq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DummyConnectorUpi {
    UpiCollect(DummyConnectorUpiCollect),
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct DummyConnectorUpiCollect {
    vpa_id: Secret<String, pii::UpiVpaMaskingStrategy>,
}

impl TryFrom<UpiCollectData> for DummyConnectorUpi {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(value: UpiCollectData) -> Result<Self, Self::Error> {
        Ok(Self::UpiCollect(DummyConnectorUpiCollect {
            vpa_id: value.vpa_id.ok_or(ConnectorError::MissingRequiredField {
                field_name: "vpa_id",
            })?,
        }))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct DummyConnectorCard {
    name: Secret<String>,
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
}

impl TryFrom<(Card, Option<Secret<String>>)> for DummyConnectorCard {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        (value, card_holder_name): (Card, Option<Secret<String>>),
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

impl TryFrom<WalletData> for DummyConnectorWallet {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(value: WalletData) -> Result<Self, Self::Error> {
        match value {
            WalletData::GooglePayRedirect(_) => Ok(Self::GooglePay),
            WalletData::PaypalRedirect(_) => Ok(Self::Paypal),
            WalletData::WeChatPayRedirect(_) => Ok(Self::WeChatPay),
            WalletData::MbWayRedirect(_) => Ok(Self::MbWay),
            WalletData::AliPayRedirect(_) => Ok(Self::AliPay),
            WalletData::AliPayHkRedirect(_) => Ok(Self::AliPayHK),
            _ => Err(ConnectorError::NotImplemented("Dummy wallet".to_string()).into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum DummyConnectorPayLater {
    Klarna,
    Affirm,
    AfterPayClearPay,
}

impl TryFrom<PayLaterData> for DummyConnectorPayLater {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(value: PayLaterData) -> Result<Self, Self::Error> {
        match value {
            PayLaterData::KlarnaRedirect { .. } => Ok(Self::Klarna),
            PayLaterData::AffirmRedirect {} => Ok(Self::Affirm),
            PayLaterData::AfterpayClearpayRedirect { .. } => Ok(Self::AfterPayClearPay),
            _ => Err(ConnectorError::NotImplemented("Dummy pay later".to_string()).into()),
        }
    }
}

impl<const T: u8> TryFrom<&DummyConnectorRouterData<&PaymentsAuthorizeRouterData>>
    for DummyConnectorPaymentsRequest<T>
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: &DummyConnectorRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let payment_method_data: Result<DummyPaymentMethodData, Self::Error> =
            match item.router_data.request.payment_method_data {
                PaymentMethodData::Card(ref req_card) => {
                    let card_holder_name = item.router_data.get_optional_billing_full_name();
                    Ok(DummyPaymentMethodData::Card(DummyConnectorCard::try_from(
                        (req_card.clone(), card_holder_name),
                    )?))
                }
                PaymentMethodData::Upi(ref req_upi_data) => match req_upi_data {
                    UpiData::UpiCollect(data) => Ok(DummyPaymentMethodData::Upi(
                        DummyConnectorUpi::try_from(data.clone())?,
                    )),
                    UpiData::UpiIntent(_) | UpiData::UpiQr(_) => {
                        Err(ConnectorError::NotImplemented("UPI flow".to_string()).into())
                    }
                },
                PaymentMethodData::Wallet(ref wallet_data) => Ok(DummyPaymentMethodData::Wallet(
                    wallet_data.clone().try_into()?,
                )),
                PaymentMethodData::PayLater(ref pay_later_data) => Ok(
                    DummyPaymentMethodData::PayLater(pay_later_data.clone().try_into()?),
                ),
                _ => Err(ConnectorError::NotImplemented("Payment methods".to_string()).into()),
            };
        Ok(Self {
            amount: item.router_data.request.minor_amount,
            currency: item.router_data.request.currency,
            payment_method_data: payment_method_data?,
            return_url: item.router_data.request.router_return_url.clone(),
            connector: Into::<DummyConnectors>::into(T),
        })
    }
}

// Auth Struct
pub struct DummyConnectorAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for DummyConnectorAuthType {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(ConnectorError::FailedToObtainAuthType.into()),
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

impl From<DummyConnectorPaymentStatus> for AttemptStatus {
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
    amount: MinorUnit,
    currency: Currency,
    created: String,
    payment_method_type: PaymentMethodType,
    next_action: Option<DummyConnectorNextAction>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PaymentMethodType {
    Card,
    Upi(DummyConnectorUpiType),
    Wallet(DummyConnectorWallet),
    PayLater(DummyConnectorPayLater),
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum DummyConnectorUpiType {
    UpiCollect,
}

impl<F, T> TryFrom<ResponseRouterData<F, PaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirection_data = item
            .response
            .next_action
            .and_then(|redirection_data| redirection_data.get_url())
            .map(|redirection_url| RedirectForm::from((redirection_url, Method::Get)));
        Ok(Self {
            status: AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
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
    pub amount: MinorUnit,
}

impl<F> TryFrom<&DummyConnectorRouterData<&RefundsRouterData<F>>> for DummyConnectorRefundRequest {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: &DummyConnectorRouterData<&RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.router_data.request.minor_refund_amount,
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum DummyRefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<DummyRefundStatus> for RefundStatus {
    fn from(item: DummyRefundStatus) -> Self {
        match item {
            DummyRefundStatus::Succeeded => Self::Success,
            DummyRefundStatus::Failed => Self::Failure,
            DummyRefundStatus::Processing => Self::Pending,
            //TODO: Review mapping
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: DummyRefundStatus,
    currency: Currency,
    created: String,
    payment_amount: MinorUnit,
    refund_amount: MinorUnit,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: RefundStatus::from(item.response.status),
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
