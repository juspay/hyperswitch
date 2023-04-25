use api_models::payments::PaymentMethodData;
use base64::Engine;
use error_stack::{IntoReport, ResultExt};
use masking::Secret;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    connector::utils::{CardData, PaymentsCancelRequestData},
    consts,
    core::errors,
    services,
    types::{self, api, storage::enums, transformers::ForeignFrom},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NexinetsPaymentsRequest {
    initial_amount: i64,
    currency: enums::Currency,
    channel: NexinetsChannel,
    product: NexinetsProduct,
    payment: Option<NexinetsPaymentDetails>,
    #[serde(rename = "async")]
    nexinets_async: NexinetsAsyncDetails,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NexinetsChannel {
    #[default]
    Ecom,
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NexinetsProduct {
    #[default]
    Creditcard,
    Paypal,
    Giropay,
    Sofort,
    Eps,
    Ideal,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum NexinetsPaymentDetails {
    Card(NexiCardDetails),
    BankRedirects(NexinetsBankRedirects),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NexiCardDetails {
    card_number: Secret<String, common_utils::pii::CardNumber>,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    verification: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NexinetsBankRedirects {
    bic: NexinetsBIC,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]

pub struct NexinetsAsyncDetails {
    pub success_url: Option<String>,
    pub cancel_url: Option<String>,
    pub failure_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub enum NexinetsBIC {
    #[serde(rename = "ABNANL2A")]
    AbnAmro,
    #[serde(rename = "ASNBNL21")]
    AsnBank,
    #[serde(rename = "BUNQNL2A")]
    Bunq,
    #[serde(rename = "INGBNL2A")]
    Ing,
    #[serde(rename = "KNABNL2H")]
    Knab,
    #[serde(rename = "RABONL2U")]
    Rabobank,
    #[serde(rename = "RBRBNL21")]
    Regiobank,
    #[serde(rename = "SNSBNL2A")]
    SnsBank,
    #[serde(rename = "TRIONL2U")]
    TriodosBank,
    #[serde(rename = "FVLBNL22")]
    VanLanschot,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for NexinetsPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let return_url = item.request.router_return_url.clone();
        let nexinets_async = NexinetsAsyncDetails {
            success_url: return_url.clone(),
            cancel_url: return_url.clone(),
            failure_url: return_url,
        };
        let (payment, product) =
            get_payment_details_and_product(&item.request.payment_method_data)?;
        Ok(Self {
            initial_amount: item.request.amount,
            currency: item.request.currency,
            channel: NexinetsChannel::Ecom,
            product,
            payment,
            nexinets_async,
        })
    }
}

// Auth Struct
pub struct NexinetsAuthType {
    pub(super) api_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for NexinetsAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => {
                let auth_key = format!("{key1}:{api_key}");
                let auth_header = format!("Basic {}", consts::BASE64_ENGINE.encode(auth_key));
                Ok(Self {
                    api_key: auth_header,
                })
            }
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)?,
        }
    }
}
// PaymentsResponse
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NexinetsPaymentStatus {
    Success,
    Pending,
    Ok,
    Failure,
    Declined,
    InProgress,
    Expired,
    Aborted,
}

impl ForeignFrom<(NexinetsPaymentStatus, NexinetsTransactionType)> for enums::AttemptStatus {
    fn foreign_from((status, method): (NexinetsPaymentStatus, NexinetsTransactionType)) -> Self {
        match status {
            NexinetsPaymentStatus::Success => match method {
                NexinetsTransactionType::Preauth => Self::Authorized,
                NexinetsTransactionType::Debit | NexinetsTransactionType::Capture => Self::Charged,
                NexinetsTransactionType::Cancel => Self::Voided,
            },
            NexinetsPaymentStatus::Declined
            | NexinetsPaymentStatus::Failure
            | NexinetsPaymentStatus::Expired
            | NexinetsPaymentStatus::Aborted => match method {
                NexinetsTransactionType::Preauth => Self::AuthorizationFailed,
                NexinetsTransactionType::Debit | NexinetsTransactionType::Capture => {
                    Self::CaptureFailed
                }
                NexinetsTransactionType::Cancel => Self::VoidFailed,
            },
            NexinetsPaymentStatus::Ok => match method {
                NexinetsTransactionType::Preauth => Self::Authorized,
                _ => Self::Pending,
            },
            NexinetsPaymentStatus::Pending => Self::AuthenticationPending,
            NexinetsPaymentStatus::InProgress => Self::Pending,
        }
    }
}

impl TryFrom<&api_models::enums::BankNames> for NexinetsBIC {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(bank: &api_models::enums::BankNames) -> Result<Self, Self::Error> {
        match bank {
            api_models::enums::BankNames::AbnAmro => Ok(Self::AbnAmro),
            api_models::enums::BankNames::AsnBank => Ok(Self::AsnBank),
            api_models::enums::BankNames::Bunq => Ok(Self::Bunq),
            api_models::enums::BankNames::Ing => Ok(Self::Ing),
            api_models::enums::BankNames::Knab => Ok(Self::Knab),
            api_models::enums::BankNames::Rabobank => Ok(Self::Rabobank),
            api_models::enums::BankNames::Regiobank => Ok(Self::Regiobank),
            api_models::enums::BankNames::SnsBank => Ok(Self::SnsBank),
            api_models::enums::BankNames::TriodosBank => Ok(Self::TriodosBank),
            api_models::enums::BankNames::VanLanschot => Ok(Self::VanLanschot),
            _ => Err(errors::ConnectorError::FlowNotSupported {
                flow: bank.to_string(),
                connector: "Nexinets".to_string(),
            }
            .into()),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexinetsPreAuthOrDebitResponse {
    order_id: String,
    transaction_type: NexinetsTransactionType,
    transactions: Vec<NexinetsTransaction>,
    redirect_url: Option<Url>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexinetsTransaction {
    pub transaction_id: String,
    #[serde(rename = "type")]
    pub transaction_type: NexinetsTransactionType,
    pub currency: enums::Currency,
    pub status: NexinetsPaymentStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NexinetsTransactionType {
    Preauth,
    Debit,
    Capture,
    Cancel,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NexinetsPaymentsMetadata {
    pub transaction_id: Option<String>,
    pub order_id: Option<String>,
    pub psync_flow: NexinetsTransactionType,
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<
            F,
            NexinetsPreAuthOrDebitResponse,
            T,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            NexinetsPreAuthOrDebitResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let transaction = match item.response.transactions.first() {
            Some(order) => order,
            _ => Err(errors::ConnectorError::ResponseHandlingFailed)?,
        };
        let connector_metadata = serde_json::to_value(NexinetsPaymentsMetadata {
            transaction_id: Some(transaction.transaction_id.clone()),
            order_id: Some(item.response.order_id.clone()),
            psync_flow: item.response.transaction_type.clone(),
        })
        .into_report()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;
        let redirection_data = item
            .response
            .redirect_url
            .map(|url| services::RedirectForm::from((url, services::Method::Get)));
        let resource_id = match item.response.transaction_type.clone() {
            NexinetsTransactionType::Preauth => types::ResponseId::NoResponseId,
            NexinetsTransactionType::Debit => {
                types::ResponseId::ConnectorTransactionId(transaction.transaction_id.clone())
            }
            _ => Err(errors::ConnectorError::ResponseHandlingFailed)?,
        };
        Ok(Self {
            status: enums::AttemptStatus::foreign_from((
                transaction.status.clone(),
                item.response.transaction_type,
            )),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id,
                redirection_data,
                mandate_reference: None,
                connector_metadata: Some(connector_metadata),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NexinetsCaptureOrVoidRequest {
    pub initial_amount: i64,
    pub currency: enums::Currency,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexinetsOrder {
    pub order_id: String,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for NexinetsCaptureOrVoidRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            initial_amount: item.request.amount_to_capture,
            currency: item.request.currency,
        })
    }
}

impl TryFrom<&types::PaymentsCancelRouterData> for NexinetsCaptureOrVoidRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            initial_amount: item.request.get_amount()?,
            currency: item.request.get_currency()?,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexinetsPaymentResponse {
    pub transaction_id: String,
    pub status: NexinetsPaymentStatus,
    pub order: NexinetsOrder,
    #[serde(rename = "type")]
    pub transaction_type: NexinetsTransactionType,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, NexinetsPaymentResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, NexinetsPaymentResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let transaction_id = Some(item.response.transaction_id.clone());
        let connector_metadata = serde_json::to_value(NexinetsPaymentsMetadata {
            transaction_id,
            order_id: Some(item.response.order.order_id),
            psync_flow: item.response.transaction_type.clone(),
        })
        .into_report()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;
        let resource_id = match item.response.transaction_type.clone() {
            NexinetsTransactionType::Debit | NexinetsTransactionType::Capture => {
                types::ResponseId::ConnectorTransactionId(item.response.transaction_id)
            }
            _ => types::ResponseId::NoResponseId,
        };
        Ok(Self {
            status: enums::AttemptStatus::foreign_from((
                item.response.status,
                item.response.transaction_type,
            )),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id,
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: Some(connector_metadata),
            }),
            ..item.data
        })
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NexinetsRefundRequest {
    pub initial_amount: i64,
    pub currency: enums::Currency,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for NexinetsRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            initial_amount: item.request.refund_amount,
            currency: item.request.currency,
        })
    }
}

// Type definition for Refund Response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexinetsRefundResponse {
    pub transaction_id: String,
    pub status: RefundStatus,
    pub order: NexinetsOrder,
    #[serde(rename = "type")]
    pub transaction_type: RefundType,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RefundStatus {
    Success,
    Ok,
    Failure,
    Declined,
    InProgress,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RefundType {
    Refund,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Success => Self::Success,
            RefundStatus::Failure | RefundStatus::Declined => Self::Failure,
            RefundStatus::InProgress | RefundStatus::Ok => Self::Pending,
        }
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, NexinetsRefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, NexinetsRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.transaction_id,
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, NexinetsRefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, NexinetsRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.transaction_id,
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct NexinetsErrorResponse {
    pub status: u16,
    pub code: u16,
    pub message: String,
    pub errors: Vec<OrderErrorDetails>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OrderErrorDetails {
    pub code: u16,
    pub message: String,
    pub field: Option<String>,
}

fn get_payment_details_and_product(
    item: &PaymentMethodData,
) -> Result<
    (Option<NexinetsPaymentDetails>, NexinetsProduct),
    error_stack::Report<errors::ConnectorError>,
> {
    match item {
        PaymentMethodData::Card(card) => {
            Ok((Some(get_card_details(card)), NexinetsProduct::Creditcard))
        }
        PaymentMethodData::Wallet(api_models::payments::WalletData::PaypalRedirect(_)) => {
            Ok((None, NexinetsProduct::Paypal))
        }
        PaymentMethodData::BankRedirect(bank_redirect) => match bank_redirect {
            api_models::payments::BankRedirectData::Eps { .. } => Ok((None, NexinetsProduct::Eps)),
            api_models::payments::BankRedirectData::Giropay { .. } => {
                Ok((None, NexinetsProduct::Giropay))
            }
            api_models::payments::BankRedirectData::Ideal { bank_name, .. } => Ok((
                Some(NexinetsPaymentDetails::BankRedirects(
                    NexinetsBankRedirects {
                        bic: NexinetsBIC::try_from(bank_name)?,
                    },
                )),
                NexinetsProduct::Ideal,
            )),
            api_models::payments::BankRedirectData::Sofort { .. } => {
                Ok((None, NexinetsProduct::Sofort))
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment methods".to_string(),
            ))?,
        },
        _ => Err(errors::ConnectorError::NotImplemented(
            "Payment methods".to_string(),
        ))?,
    }
}

fn get_card_details(req_card: &api_models::payments::Card) -> NexinetsPaymentDetails {
    NexinetsPaymentDetails::Card(NexiCardDetails {
        card_number: req_card.card_number.clone(),
        expiry_month: req_card.card_exp_month.clone(),
        expiry_year: req_card.get_card_expiry_year_2_digit(),
        verification: req_card.card_cvc.clone(),
    })
}

pub fn get_order_id(
    meta: &NexinetsPaymentsMetadata,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    let order_id = meta.order_id.clone().ok_or(
        errors::ConnectorError::MissingConnectorRelatedTransactionID {
            id: "order_id".to_string(),
        },
    )?;
    Ok(order_id)
}

pub fn get_transaction_id(
    meta: &NexinetsPaymentsMetadata,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    let transaction_id = meta.transaction_id.clone().ok_or(
        errors::ConnectorError::MissingConnectorRelatedTransactionID {
            id: "transaction_id".to_string(),
        },
    )?;
    Ok(transaction_id)
}
