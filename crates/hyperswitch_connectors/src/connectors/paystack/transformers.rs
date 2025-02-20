use common_enums::{enums, Currency};
use common_utils::{pii::Email, request::Method, types::MinorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{BankRedirectData, PaymentMethodData},
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::PaymentsAuthorizeRequestData,
};

pub struct PaystackRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for PaystackRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct PaystackEftProvider {
    provider: String,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct PaystackPaymentsRequest {
    amount: MinorUnit,
    currency: Currency,
    email: Email,
    eft: PaystackEftProvider,
}

impl TryFrom<&PaystackRouterData<&PaymentsAuthorizeRouterData>> for PaystackPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PaystackRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::BankRedirect(BankRedirectData::Eft { provider }) => {
                let email = item.router_data.request.get_email()?;
                let eft = PaystackEftProvider { provider };
                Ok(Self {
                    amount: item.amount.clone(),
                    currency: item.router_data.request.currency,
                    email,
                    eft,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

pub struct PaystackAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for PaystackAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaystackEftRedirect {
    reference: String,
    status: String,
    url: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaystackPaymentsResponseData {
    status: bool,
    message: String,
    data: PaystackEftRedirect,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum PaystackPaymentsResponse {
    PaystackPaymentsData(PaystackPaymentsResponseData),
    PaystackPaymentsError(PaystackErrorResponse),
}

impl<F, T> TryFrom<ResponseRouterData<F, PaystackPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PaystackPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let (status, response) = match item.response {
            PaystackPaymentsResponse::PaystackPaymentsData(resp) => {
                let redirection_url = Url::parse(resp.data.url.as_str())
                    .change_context(errors::ConnectorError::ParsingFailed)?;
                let redirection_data = RedirectForm::from((redirection_url, Method::Get));
                (
                    common_enums::AttemptStatus::AuthenticationPending,
                    Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(
                            resp.data.reference.clone(),
                        ),
                        redirection_data: Box::new(Some(redirection_data)),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        incremental_authorization_allowed: None,
                        charges: None,
                    }),
                )
            }
            PaystackPaymentsResponse::PaystackPaymentsError(err) => {
                let err_msg = get_error_message(err.clone());
                (
                    common_enums::AttemptStatus::Failure,
                    Err(ErrorResponse {
                        code: err.code,
                        message: err_msg.clone(),
                        reason: Some(err_msg.clone()),
                        attempt_status: None,
                        connector_transaction_id: None,
                        status_code: item.http_code,
                    }),
                )
            }
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PaystackPSyncStatus {
    Abandoned,
    Failed,
    Ongoing,
    Pending,
    Processing,
    Queued,
    Reversed,
    Success,
}

impl From<PaystackPSyncStatus> for common_enums::AttemptStatus {
    fn from(item: PaystackPSyncStatus) -> Self {
        match item {
            PaystackPSyncStatus::Success => Self::Charged,
            PaystackPSyncStatus::Abandoned
            | PaystackPSyncStatus::Ongoing
            | PaystackPSyncStatus::Pending
            | PaystackPSyncStatus::Processing
            | PaystackPSyncStatus::Queued => Self::AuthenticationPending,
            PaystackPSyncStatus::Failed => Self::Failure,
            PaystackPSyncStatus::Reversed => Self::AutoRefunded,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaystackPSyncData {
    status: PaystackPSyncStatus,
    reference: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaystackPSyncResponseData {
    status: bool,
    message: String,
    data: PaystackPSyncData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum PaystackPSyncResponse {
    PaystackPSyncData(PaystackPSyncResponseData),
    PaystackPSyncWebhook(PaystackPaymentWebhookData),
    PaystackPSyncError(PaystackErrorResponse),
}

impl<F, T> TryFrom<ResponseRouterData<F, PaystackPSyncResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PaystackPSyncResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            PaystackPSyncResponse::PaystackPSyncData(resp) => Ok(Self {
                status: common_enums::AttemptStatus::from(resp.data.status),
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(resp.data.reference.clone()),
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
            PaystackPSyncResponse::PaystackPSyncWebhook(resp) => Ok(Self {
                status: common_enums::AttemptStatus::from(resp.status),
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(resp.reference.clone()),
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
            PaystackPSyncResponse::PaystackPSyncError(err) => {
                let err_msg = get_error_message(err.clone());
                Ok(Self {
                    response: Err(ErrorResponse {
                        code: err.code,
                        message: err_msg.clone(),
                        reason: Some(err_msg.clone()),
                        attempt_status: None,
                        connector_transaction_id: None,
                        status_code: item.http_code,
                    }),
                    ..item.data
                })
            }
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct PaystackRefundRequest {
    pub transaction: String,
    pub amount: MinorUnit,
}

impl<F> TryFrom<&PaystackRouterData<&RefundsRouterData<F>>> for PaystackRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaystackRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction: item.router_data.request.connector_transaction_id.clone(),
            amount: item.amount.to_owned(),
        })
    }
}

#[derive(Debug, Serialize, Default, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PaystackRefundStatus {
    Processed,
    Failed,
    #[default]
    Processing,
    Pending,
}

impl From<PaystackRefundStatus> for enums::RefundStatus {
    fn from(item: PaystackRefundStatus) -> Self {
        match item {
            PaystackRefundStatus::Processed => Self::Success,
            PaystackRefundStatus::Failed => Self::Failure,
            PaystackRefundStatus::Processing | PaystackRefundStatus::Pending => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaystackRefundsData {
    status: PaystackRefundStatus,
    id: i64,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaystackRefundsResponseData {
    status: bool,
    message: String,
    data: PaystackRefundsData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum PaystackRefundsResponse {
    PaystackRefundsData(PaystackRefundsResponseData),
    PaystackRSyncWebhook(PaystackRefundWebhookData),
    PaystackRefundsError(PaystackErrorResponse),
}

impl TryFrom<RefundsResponseRouterData<Execute, PaystackRefundsResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, PaystackRefundsResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            PaystackRefundsResponse::PaystackRefundsData(resp) => Ok(Self {
                response: Ok(RefundsResponseData {
                    connector_refund_id: resp.data.id.to_string(),
                    refund_status: enums::RefundStatus::from(resp.data.status),
                }),
                ..item.data
            }),
            PaystackRefundsResponse::PaystackRSyncWebhook(resp) => Ok(Self {
                response: Ok(RefundsResponseData {
                    connector_refund_id: resp.id,
                    refund_status: enums::RefundStatus::from(resp.status),
                }),
                ..item.data
            }),
            PaystackRefundsResponse::PaystackRefundsError(err) => {
                let err_msg = get_error_message(err.clone());
                Ok(Self {
                    response: Err(ErrorResponse {
                        code: err.code,
                        message: err_msg.clone(),
                        reason: Some(err_msg.clone()),
                        attempt_status: None,
                        connector_transaction_id: None,
                        status_code: item.http_code,
                    }),
                    ..item.data
                })
            }
        }
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, PaystackRefundsResponse>>
    for RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, PaystackRefundsResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            PaystackRefundsResponse::PaystackRefundsData(resp) => Ok(Self {
                response: Ok(RefundsResponseData {
                    connector_refund_id: resp.data.id.to_string(),
                    refund_status: enums::RefundStatus::from(resp.data.status),
                }),
                ..item.data
            }),
            PaystackRefundsResponse::PaystackRSyncWebhook(resp) => Ok(Self {
                response: Ok(RefundsResponseData {
                    connector_refund_id: resp.id,
                    refund_status: enums::RefundStatus::from(resp.status),
                }),
                ..item.data
            }),
            PaystackRefundsResponse::PaystackRefundsError(err) => {
                let err_msg = get_error_message(err.clone());
                Ok(Self {
                    response: Err(ErrorResponse {
                        code: err.code,
                        message: err_msg.clone(),
                        reason: Some(err_msg.clone()),
                        attempt_status: None,
                        connector_transaction_id: None,
                        status_code: item.http_code,
                    }),
                    ..item.data
                })
            }
        }
    }
}

#[derive(Default, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PaystackErrorResponse {
    pub status: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
    pub meta: serde_json::Value,
    pub code: String,
}

pub fn get_error_message(response: PaystackErrorResponse) -> String {
    if let Some(serde_json::Value::Object(err_map)) = response.data {
        err_map.get("message").map(|msg| msg.clone().to_string())
    } else {
        None
    }
    .unwrap_or(response.message)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PaystackPaymentWebhookData {
    pub status: PaystackPSyncStatus,
    pub reference: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PaystackRefundWebhookData {
    pub status: PaystackRefundStatus,
    pub id: String,
    pub transaction_reference: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum PaystackWebhookEventData {
    Payment(PaystackPaymentWebhookData),
    Refund(PaystackRefundWebhookData),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PaystackWebhookData {
    pub event: String,
    pub data: PaystackWebhookEventData,
}

impl From<PaystackWebhookEventData> for api_models::webhooks::IncomingWebhookEvent {
    fn from(item: PaystackWebhookEventData) -> Self {
        match item {
            PaystackWebhookEventData::Payment(payment_data) => match payment_data.status {
                PaystackPSyncStatus::Success => Self::PaymentIntentSuccess,
                PaystackPSyncStatus::Failed => Self::PaymentIntentFailure,
                PaystackPSyncStatus::Abandoned
                | PaystackPSyncStatus::Ongoing
                | PaystackPSyncStatus::Pending
                | PaystackPSyncStatus::Processing
                | PaystackPSyncStatus::Queued => Self::PaymentIntentProcessing,
                PaystackPSyncStatus::Reversed => Self::EventNotSupported,
            },
            PaystackWebhookEventData::Refund(refund_data) => match refund_data.status {
                PaystackRefundStatus::Processed => Self::RefundSuccess,
                PaystackRefundStatus::Failed => Self::RefundFailure,
                PaystackRefundStatus::Processing | PaystackRefundStatus::Pending => {
                    Self::EventNotSupported
                }
            },
        }
    }
}
