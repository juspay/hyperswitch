use masking::Secret;
use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::PaymentsAuthorizeRequestData,
    core::errors,
    services,
    types::{self, api, storage::enums, ConnectorAuthType},
};

#[derive(Debug, Serialize)]
pub struct BitpayRouterData<T> {
    pub amount: i64,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for BitpayRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        (_currency_unit, _currency, amount, router_data): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data,
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionSpeed {
    Low,
    #[default]
    Medium,
    High,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BitpayPaymentsRequest {
    price: i64,
    currency: String,
    #[serde(rename = "redirectURL")]
    redirect_url: String,
    #[serde(rename = "notificationURL")]
    notification_url: String,
    transaction_speed: TransactionSpeed,
    token: Secret<String>,
    order_id: String,
}

impl TryFrom<&BitpayRouterData<&types::PaymentsAuthorizeRouterData>> for BitpayPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &BitpayRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        get_crypto_specific_payment_data(item)
    }
}

// Auth Struct
pub struct BitpayAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for BitpayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BitpayPaymentStatus {
    #[default]
    New,
    Paid,
    Confirmed,
    Complete,
    Expired,
    Invalid,
}

impl From<BitpayPaymentStatus> for enums::AttemptStatus {
    fn from(item: BitpayPaymentStatus) -> Self {
        match item {
            BitpayPaymentStatus::New => Self::AuthenticationPending,
            BitpayPaymentStatus::Complete | BitpayPaymentStatus::Confirmed => Self::Charged,
            BitpayPaymentStatus::Expired => Self::Failure,
            _ => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExceptionStatus {
    #[default]
    Unit,
    Bool(bool),
    String(String),
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BitpayPaymentResponseData {
    pub url: Option<Url>,
    pub status: BitpayPaymentStatus,
    pub price: i64,
    pub currency: String,
    pub amount_paid: i64,
    pub invoice_time: Option<i64>,
    pub rate_refresh_time: Option<i64>,
    pub expiration_time: Option<i64>,
    pub current_time: Option<i64>,
    pub id: String,
    pub low_fee_detected: Option<bool>,
    pub display_amount_paid: Option<String>,
    pub exception_status: ExceptionStatus,
    pub redirect_url: Option<String>,
    pub refund_address_request_pending: Option<bool>,
    pub merchant_name: Option<String>,
    pub token: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct BitpayPaymentsResponse {
    data: BitpayPaymentResponseData,
    facade: Option<String>,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, BitpayPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, BitpayPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirection_data = item
            .response
            .data
            .url
            .map(|x| services::RedirectForm::from((x, services::Method::Get)));
        let connector_id = types::ResponseId::ConnectorTransactionId(item.response.data.id);
        let attempt_status = item.response.data.status;
        Ok(Self {
            status: enums::AttemptStatus::from(attempt_status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: connector_id,
                redirection_data,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
            }),
            ..item.data
        })
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct BitpayRefundRequest {
    pub amount: i64,
}

impl<F> TryFrom<&BitpayRouterData<&types::RefundsRouterData<F>>> for BitpayRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &BitpayRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.router_data.request.refund_amount,
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
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

#[derive(Debug, Deserialize)]
pub struct BitpayErrorResponse {
    pub error: String,
    pub code: Option<String>,
    pub message: Option<String>,
}

fn get_crypto_specific_payment_data(
    item: &BitpayRouterData<&types::PaymentsAuthorizeRouterData>,
) -> Result<BitpayPaymentsRequest, error_stack::Report<errors::ConnectorError>> {
    let price = item.amount;
    let currency = item.router_data.request.currency.to_string();
    let redirect_url = item.router_data.request.get_return_url()?;
    let notification_url = item.router_data.request.get_webhook_url()?;
    let transaction_speed = TransactionSpeed::Medium;
    let auth_type = item.router_data.connector_auth_type.clone();
    let token = match auth_type {
        ConnectorAuthType::HeaderKey { api_key } => api_key,
        _ => String::default().into(),
    };
    let order_id = item.router_data.connector_request_reference_id.clone();

    Ok(BitpayPaymentsRequest {
        price,
        currency,
        redirect_url,
        notification_url,
        transaction_speed,
        token,
        order_id,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BitpayWebhookDetails {
    pub event: Event,
    pub data: BitpayPaymentResponseData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    pub code: i64,
    pub name: WebhookEventType,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum WebhookEventType {
    #[serde(rename = "invoice_paidInFull")]
    Paid,
    #[serde(rename = "invoice_confirmed")]
    Confirmed,
    #[serde(rename = "invoice_completed")]
    Completed,
    #[serde(rename = "invoice_expired")]
    Expired,
    #[serde(rename = "invoice_failedToConfirm")]
    Invalid,
    #[serde(rename = "invoice_declined")]
    Declined,
    #[serde(rename = "invoice_refundComplete")]
    Refunded,
    #[serde(rename = "invoice_manuallyNotified")]
    Resent,
    #[serde(other)]
    Unknown,
}
