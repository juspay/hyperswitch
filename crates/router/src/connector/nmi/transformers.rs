use cards::CardNumber;
use common_utils::ext_traits::XmlExt;
use error_stack::{IntoReport, Report, ResultExt};
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, PaymentsAuthorizeRequestData},
    core::errors,
    types::{self, api, storage::enums, transformers::ForeignFrom, ConnectorAuthType},
};

type Error = Report<errors::ConnectorError>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Auth,
    Capture,
    Refund,
    Sale,
    Validate,
    Void,
}

pub struct NmiAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for NmiAuthType {
    type Error = Error;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = auth_type {
            Ok(Self {
                api_key: api_key.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NmiRouterData<T> {
    pub amount: f64,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for NmiRouterData<T>
{
    type Error = Report<errors::ConnectorError>;

    fn try_from(
        (_currency_unit, currency, amount, router_data): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: utils::to_currency_base_unit_asf64(amount, currency)?,
            router_data,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct NmiPaymentsRequest {
    #[serde(rename = "type")]
    transaction_type: TransactionType,
    amount: f64,
    security_key: Secret<String>,
    currency: enums::Currency,
    #[serde(flatten)]
    payment_method: PaymentMethod,
    orderid: String,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PaymentMethod {
    Card(Box<CardData>),
    GPay(Box<GooglePayData>),
    ApplePay(Box<ApplePayData>),
}

#[derive(Debug, Serialize)]
pub struct CardData {
    ccnumber: CardNumber,
    ccexp: Secret<String>,
    cvv: Secret<String>,
}

#[derive(Debug, Serialize)]
pub struct GooglePayData {
    googlepay_payment_data: Secret<String>,
}

#[derive(Debug, Serialize)]
pub struct ApplePayData {
    applepay_payment_data: Secret<String>,
}

impl TryFrom<&NmiRouterData<&types::PaymentsAuthorizeRouterData>> for NmiPaymentsRequest {
    type Error = Error;
    fn try_from(
        item: &NmiRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let transaction_type = match item.router_data.request.is_auto_capture()? {
            true => TransactionType::Sale,
            false => TransactionType::Auth,
        };
        let auth_type: NmiAuthType = (&item.router_data.connector_auth_type).try_into()?;
        let amount = item.amount;
        let payment_method =
            PaymentMethod::try_from(&item.router_data.request.payment_method_data)?;

        Ok(Self {
            transaction_type,
            security_key: auth_type.api_key,
            amount,
            currency: item.router_data.request.currency,
            payment_method,
            orderid: item.router_data.connector_request_reference_id.clone(),
        })
    }
}

impl TryFrom<&api_models::payments::PaymentMethodData> for PaymentMethod {
    type Error = Error;
    fn try_from(
        payment_method_data: &api_models::payments::PaymentMethodData,
    ) -> Result<Self, Self::Error> {
        match &payment_method_data {
            api::PaymentMethodData::Card(ref card) => Ok(Self::from(card)),
            api::PaymentMethodData::Wallet(ref wallet_type) => match wallet_type {
                api_models::payments::WalletData::GooglePay(ref googlepay_data) => {
                    Ok(Self::from(googlepay_data))
                }
                api_models::payments::WalletData::ApplePay(ref applepay_data) => {
                    Ok(Self::from(applepay_data))
                }
                api_models::payments::WalletData::AliPayQr(_)
                | api_models::payments::WalletData::AliPayRedirect(_)
                | api_models::payments::WalletData::AliPayHkRedirect(_)
                | api_models::payments::WalletData::MomoRedirect(_)
                | api_models::payments::WalletData::KakaoPayRedirect(_)
                | api_models::payments::WalletData::GoPayRedirect(_)
                | api_models::payments::WalletData::GcashRedirect(_)
                | api_models::payments::WalletData::ApplePayRedirect(_)
                | api_models::payments::WalletData::ApplePayThirdPartySdk(_)
                | api_models::payments::WalletData::DanaRedirect {}
                | api_models::payments::WalletData::GooglePayRedirect(_)
                | api_models::payments::WalletData::GooglePayThirdPartySdk(_)
                | api_models::payments::WalletData::MbWayRedirect(_)
                | api_models::payments::WalletData::MobilePayRedirect(_)
                | api_models::payments::WalletData::PaypalRedirect(_)
                | api_models::payments::WalletData::PaypalSdk(_)
                | api_models::payments::WalletData::SamsungPay(_)
                | api_models::payments::WalletData::TwintRedirect {}
                | api_models::payments::WalletData::VippsRedirect {}
                | api_models::payments::WalletData::TouchNGoRedirect(_)
                | api_models::payments::WalletData::WeChatPayRedirect(_)
                | api_models::payments::WalletData::WeChatPayQr(_)
                | api_models::payments::WalletData::CashappQr(_)
                | api_models::payments::WalletData::SwishQr(_) => {
                    Err(errors::ConnectorError::NotSupported {
                        message: utils::SELECTED_PAYMENT_METHOD.to_string(),
                        connector: "nmi",
                    })
                    .into_report()
                }
            },
            api::PaymentMethodData::CardRedirect(_)
            | api::PaymentMethodData::PayLater(_)
            | api::PaymentMethodData::BankRedirect(_)
            | api::PaymentMethodData::BankDebit(_)
            | api::PaymentMethodData::BankTransfer(_)
            | api::PaymentMethodData::Crypto(_)
            | api::PaymentMethodData::MandatePayment
            | api::PaymentMethodData::Reward
            | api::PaymentMethodData::Upi(_)
            | api::PaymentMethodData::Voucher(_)
            | api::PaymentMethodData::GiftCard(_) => Err(errors::ConnectorError::NotSupported {
                message: utils::SELECTED_PAYMENT_METHOD.to_string(),
                connector: "nmi",
            })
            .into_report(),
        }
    }
}

impl From<&api_models::payments::Card> for PaymentMethod {
    fn from(card: &api_models::payments::Card) -> Self {
        let ccexp = utils::CardData::get_card_expiry_month_year_2_digit_with_delimiter(
            card,
            "".to_string(),
        );
        let card = CardData {
            ccnumber: card.card_number.clone(),
            ccexp,
            cvv: card.card_cvc.clone(),
        };
        Self::Card(Box::new(card))
    }
}

impl From<&api_models::payments::GooglePayWalletData> for PaymentMethod {
    fn from(wallet_data: &api_models::payments::GooglePayWalletData) -> Self {
        let gpay_data = GooglePayData {
            googlepay_payment_data: Secret::new(wallet_data.tokenization_data.token.clone()),
        };
        Self::GPay(Box::new(gpay_data))
    }
}

impl From<&api_models::payments::ApplePayWalletData> for PaymentMethod {
    fn from(wallet_data: &api_models::payments::ApplePayWalletData) -> Self {
        let apple_pay_data = ApplePayData {
            applepay_payment_data: Secret::new(wallet_data.payment_data.clone()),
        };
        Self::ApplePay(Box::new(apple_pay_data))
    }
}

impl TryFrom<&types::SetupMandateRouterData> for NmiPaymentsRequest {
    type Error = Error;
    fn try_from(item: &types::SetupMandateRouterData) -> Result<Self, Self::Error> {
        let auth_type: NmiAuthType = (&item.connector_auth_type).try_into()?;
        let payment_method = PaymentMethod::try_from(&item.request.payment_method_data)?;
        Ok(Self {
            transaction_type: TransactionType::Validate,
            security_key: auth_type.api_key,
            amount: 0.0,
            currency: item.request.currency,
            payment_method,
            orderid: item.connector_request_reference_id.clone(),
        })
    }
}

#[derive(Debug, Serialize)]
pub struct NmiSyncRequest {
    pub transaction_id: String,
    pub security_key: Secret<String>,
}

impl TryFrom<&types::PaymentsSyncRouterData> for NmiSyncRequest {
    type Error = Error;
    fn try_from(item: &types::PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let auth = NmiAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            security_key: auth.api_key,
            transaction_id: item
                .request
                .connector_transaction_id
                .get_connector_transaction_id()
                .change_context(errors::ConnectorError::MissingConnectorTransactionID)?,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct NmiCaptureRequest {
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub security_key: Secret<String>,
    pub transactionid: String,
    pub amount: Option<f64>,
}

impl TryFrom<&NmiRouterData<&types::PaymentsCaptureRouterData>> for NmiCaptureRequest {
    type Error = Error;
    fn try_from(
        item: &NmiRouterData<&types::PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth = NmiAuthType::try_from(&item.router_data.connector_auth_type)?;
        Ok(Self {
            transaction_type: TransactionType::Capture,
            security_key: auth.api_key,
            transactionid: item.router_data.request.connector_transaction_id.clone(),
            amount: Some(item.amount),
        })
    }
}

impl
    TryFrom<
        types::ResponseRouterData<
            api::Capture,
            StandardResponse,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: types::ResponseRouterData<
            api::Capture,
            StandardResponse,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let (response, status) = match item.response.response {
            Response::Approved => (
                Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        item.response.transactionid,
                    ),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                }),
                enums::AttemptStatus::CaptureInitiated,
            ),
            Response::Declined | Response::Error => (
                Err(types::ErrorResponse::foreign_from((
                    item.response,
                    item.http_code,
                ))),
                enums::AttemptStatus::CaptureFailed,
            ),
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct NmiCancelRequest {
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub security_key: Secret<String>,
    pub transactionid: String,
    pub void_reason: Option<String>,
}

impl TryFrom<&types::PaymentsCancelRouterData> for NmiCancelRequest {
    type Error = Error;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let auth = NmiAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            transaction_type: TransactionType::Void,
            security_key: auth.api_key,
            transactionid: item.request.connector_transaction_id.clone(),
            void_reason: item.request.cancellation_reason.clone(),
        })
    }
}

#[derive(Debug, Deserialize)]
pub enum Response {
    #[serde(alias = "1")]
    Approved,
    #[serde(alias = "2")]
    Declined,
    #[serde(alias = "3")]
    Error,
}

#[derive(Debug, Deserialize)]
pub struct StandardResponse {
    pub response: Response,
    pub responsetext: String,
    pub authcode: Option<String>,
    pub transactionid: String,
    pub avsresponse: Option<String>,
    pub cvvresponse: Option<String>,
    pub orderid: String,
    pub response_code: String,
}

impl<T>
    TryFrom<
        types::ResponseRouterData<
            api::SetupMandate,
            StandardResponse,
            T,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<api::SetupMandate, T, types::PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: types::ResponseRouterData<
            api::SetupMandate,
            StandardResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let (response, status) = match item.response.response {
            Response::Approved => (
                Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        item.response.transactionid,
                    ),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                }),
                enums::AttemptStatus::Charged,
            ),
            Response::Declined | Response::Error => (
                Err(types::ErrorResponse::foreign_from((
                    item.response,
                    item.http_code,
                ))),
                enums::AttemptStatus::Failure,
            ),
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

impl ForeignFrom<(StandardResponse, u16)> for types::ErrorResponse {
    fn foreign_from((response, http_code): (StandardResponse, u16)) -> Self {
        Self {
            code: response.response_code,
            message: response.responsetext,
            reason: None,
            status_code: http_code,
        }
    }
}

impl TryFrom<types::PaymentsResponseRouterData<StandardResponse>>
    for types::RouterData<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: types::ResponseRouterData<
            api::Authorize,
            StandardResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let (response, status) = match item.response.response {
            Response::Approved => (
                Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        item.response.transactionid,
                    ),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                }),
                if let Some(diesel_models::enums::CaptureMethod::Automatic) =
                    item.data.request.capture_method
                {
                    enums::AttemptStatus::CaptureInitiated
                } else {
                    enums::AttemptStatus::Authorizing
                },
            ),
            Response::Declined | Response::Error => (
                Err(types::ErrorResponse::foreign_from((
                    item.response,
                    item.http_code,
                ))),
                enums::AttemptStatus::Failure,
            ),
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

impl<T>
    TryFrom<types::ResponseRouterData<api::Void, StandardResponse, T, types::PaymentsResponseData>>
    for types::RouterData<api::Void, T, types::PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: types::ResponseRouterData<
            api::Void,
            StandardResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let (response, status) = match item.response.response {
            Response::Approved => (
                Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        item.response.transactionid,
                    ),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                }),
                enums::AttemptStatus::VoidInitiated,
            ),
            Response::Declined | Response::Error => (
                Err(types::ErrorResponse::foreign_from((
                    item.response,
                    item.http_code,
                ))),
                enums::AttemptStatus::VoidFailed,
            ),
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NmiStatus {
    Abandoned,
    Cancelled,
    Pendingsettlement,
    Pending,
    Failed,
    Complete,
    InProgress,
    Unknown,
}

impl TryFrom<types::PaymentsSyncResponseRouterData<types::Response>>
    for types::PaymentsSyncRouterData
{
    type Error = Error;
    fn try_from(
        item: types::PaymentsSyncResponseRouterData<types::Response>,
    ) -> Result<Self, Self::Error> {
        let response = SyncResponse::try_from(item.response.response.to_vec())?;
        Ok(Self {
            status: enums::AttemptStatus::from(NmiStatus::from(response.transaction.condition)),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    response.transaction.transaction_id,
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
            }),
            ..item.data
        })
    }
}

impl TryFrom<Vec<u8>> for SyncResponse {
    type Error = Error;
    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let query_response = String::from_utf8(bytes)
            .into_report()
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        query_response
            .parse_xml::<Self>()
            .into_report()
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)
    }
}

impl From<NmiStatus> for enums::AttemptStatus {
    fn from(item: NmiStatus) -> Self {
        match item {
            NmiStatus::Abandoned => Self::AuthenticationFailed,
            NmiStatus::Cancelled => Self::Voided,
            NmiStatus::Pending => Self::Authorized,
            NmiStatus::Pendingsettlement | NmiStatus::Complete => Self::Charged,
            NmiStatus::InProgress => Self::AuthenticationPending,
            NmiStatus::Failed | NmiStatus::Unknown => Self::Failure,
        }
    }
}

// REFUND :
#[derive(Debug, Serialize)]
pub struct NmiRefundRequest {
    #[serde(rename = "type")]
    transaction_type: TransactionType,
    security_key: Secret<String>,
    transactionid: String,
    amount: f64,
}

impl<F> TryFrom<&NmiRouterData<&types::RefundsRouterData<F>>> for NmiRefundRequest {
    type Error = Error;
    fn try_from(item: &NmiRouterData<&types::RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let auth_type: NmiAuthType = (&item.router_data.connector_auth_type).try_into()?;
        Ok(Self {
            transaction_type: TransactionType::Refund,
            security_key: auth_type.api_key,
            transactionid: item.router_data.request.connector_transaction_id.clone(),
            amount: item.amount,
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, StandardResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = Error;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, StandardResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.response);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.transactionid,
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Capture, StandardResponse>>
    for types::RefundsRouterData<api::Capture>
{
    type Error = Error;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Capture, StandardResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.response);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.transactionid,
                refund_status,
            }),
            ..item.data
        })
    }
}

impl From<Response> for enums::RefundStatus {
    fn from(item: Response) -> Self {
        match item {
            Response::Approved => Self::Pending,
            Response::Declined | Response::Error => Self::Failure,
        }
    }
}

impl TryFrom<&types::RefundSyncRouterData> for NmiSyncRequest {
    type Error = Error;
    fn try_from(item: &types::RefundSyncRouterData) -> Result<Self, Self::Error> {
        let auth = NmiAuthType::try_from(&item.connector_auth_type)?;
        let transaction_id = item
            .request
            .connector_refund_id
            .clone()
            .ok_or(errors::ConnectorError::MissingConnectorRefundID)?;

        Ok(Self {
            security_key: auth.api_key,
            transaction_id,
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, types::Response>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = Error;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, types::Response>,
    ) -> Result<Self, Self::Error> {
        let response = SyncResponse::try_from(item.response.response.to_vec())?;
        let refund_status =
            enums::RefundStatus::from(NmiStatus::from(response.transaction.condition));
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: response.transaction.transaction_id,
                refund_status,
            }),
            ..item.data
        })
    }
}

impl From<NmiStatus> for enums::RefundStatus {
    fn from(item: NmiStatus) -> Self {
        match item {
            NmiStatus::Abandoned
            | NmiStatus::Cancelled
            | NmiStatus::Failed
            | NmiStatus::Unknown => Self::Failure,
            NmiStatus::Pending | NmiStatus::InProgress => Self::Pending,
            NmiStatus::Pendingsettlement | NmiStatus::Complete => Self::Success,
        }
    }
}

impl From<String> for NmiStatus {
    fn from(value: String) -> Self {
        match value.as_str() {
            "abandoned" => Self::Abandoned,
            "canceled" => Self::Cancelled,
            "in_progress" => Self::InProgress,
            "pendingsettlement" => Self::Pendingsettlement,
            "complete" => Self::Complete,
            "failed" => Self::Failed,
            "unknown" => Self::Unknown,
            // Other than above values only pending is possible, since value is a string handling this as default
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SyncTransactionResponse {
    transaction_id: String,
    condition: String,
}

#[derive(Debug, Deserialize)]
struct SyncResponse {
    transaction: SyncTransactionResponse,
}
