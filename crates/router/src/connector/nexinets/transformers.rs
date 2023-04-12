use base64::Engine;
use error_stack::{IntoReport, ResultExt};
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::CardData,
    consts,
    core::errors,
    types::{self, api, storage::enums, transformers::ForeignFrom},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NexinetsPaymentsRequest {
    initial_amount: i64,
    currency: String,
    channel: NexinetsChannel,
    product: NexinetsProduct,
    payment: NexinetsPaymentDetails,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NexinetsChannel {
    #[default]
    Moto,
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NexinetsProduct {
    #[default]
    Creditcard,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NexinetsPaymentDetails {
    card_number: Secret<String, common_utils::pii::CardNumber>,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    verification: Secret<String>,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for NexinetsPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data {
            api::PaymentMethodData::Card(ref req_card) => {
                let payment = NexinetsPaymentDetails {
                    card_number: req_card.card_number.clone(),
                    expiry_month: req_card.card_exp_month.clone(),
                    expiry_year: req_card.clone().get_card_expiry_year_2_digit(),
                    verification: req_card.card_cvc.clone(),
                };
                Ok(Self {
                    initial_amount: item.request.amount,
                    currency: item.request.currency.to_string(),
                    channel: NexinetsChannel::Moto,
                    product: NexinetsProduct::Creditcard,
                    payment,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

// Auth Struct
pub struct NexinetsAuthType {
    pub(super) api_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for NexinetsAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::BodyKey { api_key, key1 } = auth_type {
            let auth_key = format!("{key1}:{api_key}");
            let auth_header = format!("Basic {}", consts::BASE64_ENGINE.encode(auth_key));
            Ok(Self {
                api_key: auth_header,
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
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
}

impl ForeignFrom<(NexinetsPaymentStatus, NexinetsTransactionType)> for enums::AttemptStatus {
    fn foreign_from((status, method): (NexinetsPaymentStatus, NexinetsTransactionType)) -> Self {
        match status {
            NexinetsPaymentStatus::Success | NexinetsPaymentStatus::Ok => match method {
                NexinetsTransactionType::Preauth => enums::AttemptStatus::Authorized,
                NexinetsTransactionType::Debit | NexinetsTransactionType::Capture => {
                    enums::AttemptStatus::Charged
                }
                NexinetsTransactionType::Cancel => enums::AttemptStatus::Voided,
            },
            NexinetsPaymentStatus::Declined | NexinetsPaymentStatus::Failure => match method {
                NexinetsTransactionType::Preauth => enums::AttemptStatus::AuthorizationFailed,
                NexinetsTransactionType::Debit | NexinetsTransactionType::Capture => {
                    enums::AttemptStatus::CaptureFailed
                }
                NexinetsTransactionType::Cancel => enums::AttemptStatus::VoidFailed,
            },
            _ => enums::AttemptStatus::Pending,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexinetsPreAuthOrDebitResponse {
    order_id: String,
    transaction_type: NexinetsTransactionType,
    transactions: Vec<NexinetsTransaction>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexinetsTransaction {
    pub transaction_id: String,
    #[serde(rename = "type")]
    pub transaction_type: NexinetsTransactionType,
    pub currency: String,
    pub status: NexinetsPaymentStatus,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NexinetsTransactionType {
    #[default]
    Preauth,
    Debit,
    Capture,
    Cancel,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NexinetsPaymentsMetadata {
    pub transaction_id: Option<String>,
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
        })
        .into_report()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;
        Ok(Self {
            status: enums::AttemptStatus::foreign_from((
                transaction.status.clone(),
                item.response.transaction_type,
            )),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.order_id),
                redirection_data: None,
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
    pub currency: String,
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
            currency: item.request.currency.to_string(),
        })
    }
}

impl TryFrom<&types::PaymentsCancelRouterData> for NexinetsCaptureOrVoidRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            initial_amount: item
                .request
                .amount
                .ok_or(errors::ConnectorError::RequestEncodingFailed)?,
            currency: item
                .request
                .currency
                .ok_or(errors::ConnectorError::RequestEncodingFailed)?
                .to_string(),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexinetsPaymentResponse {
    pub transaction_id: Option<String>,
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
        let transaction_id = item.response.transaction_id;
        let connector_metadata = serde_json::to_value(NexinetsPaymentsMetadata { transaction_id })
            .into_report()
            .change_context(errors::ConnectorError::ResponseHandlingFailed)?;
        Ok(Self {
            status: enums::AttemptStatus::foreign_from((
                item.response.status,
                item.response.transaction_type,
            )),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.order.order_id,
                ),
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
    pub currency: String,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for NexinetsRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            initial_amount: item.request.refund_amount,
            currency: item.request.currency.to_string(),
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
    Pending,
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
            RefundStatus::Success | RefundStatus::Ok => Self::Success,
            RefundStatus::Failure | RefundStatus::Declined => Self::Failure,
            RefundStatus::InProgress | RefundStatus::Pending => Self::Pending,
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

#[derive(Debug, Deserialize, Clone)]
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
    pub field: String,
}
