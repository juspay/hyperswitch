use common_enums::enums;
use common_utils::{
    request::Method,
    types::{MinorUnit, StringMinorUnit, StringMinorUnitForConnector},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{BankDebitData, PaymentMethodData},
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
    utils::{self, PaymentsAuthorizeRequestData, RouterData as _},
};
pub struct InespayRouterData<T> {
    pub amount: StringMinorUnit,
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for InespayRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InespayPaymentsRequest {
    description: String,
    amount: StringMinorUnit,
    reference: String,
    debtor_account: Option<Secret<String>>,
    success_link_redirect: Option<String>,
    notif_url: Option<String>,
}

impl TryFrom<&InespayRouterData<&PaymentsAuthorizeRouterData>> for InespayPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &InespayRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::BankDebit(BankDebitData::SepaBankDebit { iban, .. }) => {
                let order_id = item.router_data.connector_request_reference_id.clone();
                let webhook_url = item.router_data.request.get_webhook_url()?;
                let return_url = item.router_data.request.get_router_return_url()?;
                Ok(Self {
                    description: item.router_data.get_description()?,
                    amount: item.amount.clone(),
                    reference: order_id,
                    debtor_account: Some(iban),
                    success_link_redirect: Some(return_url),
                    notif_url: Some(webhook_url),
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

pub struct InespayAuthType {
    pub(super) api_key: Secret<String>,
    pub authorization: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for InespayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_owned(),
                authorization: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InespayPaymentsResponseData {
    status: String,
    status_desc: String,
    single_payin_id: String,
    single_payin_link: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum InespayPaymentsResponse {
    InespayPaymentsData(InespayPaymentsResponseData),
    InespayPaymentsError(InespayErrorResponse),
}

impl<F, T> TryFrom<ResponseRouterData<F, InespayPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, InespayPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let (status, response) = match item.response {
            InespayPaymentsResponse::InespayPaymentsData(data) => {
                let redirection_url = Url::parse(data.single_payin_link.as_str())
                    .change_context(errors::ConnectorError::ParsingFailed)?;
                let redirection_data = RedirectForm::from((redirection_url, Method::Get));

                (
                    common_enums::AttemptStatus::AuthenticationPending,
                    Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(
                            data.single_payin_id.clone(),
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
            InespayPaymentsResponse::InespayPaymentsError(data) => (
                common_enums::AttemptStatus::Failure,
                Err(ErrorResponse {
                    code: data.status.clone(),
                    message: data.status_desc.clone(),
                    reason: Some(data.status_desc.clone()),
                    attempt_status: None,
                    connector_transaction_id: None,
                    status_code: item.http_code,
                }),
            ),
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InespayPSyncStatus {
    Ok,
    Created,
    Opened,
    BankSelected,
    Initiated,
    Pending,
    Aborted,
    Unfinished,
    Rejected,
    Cancelled,
    PartiallyAccepted,
    Failed,
    Settled,
    PartRefunded,
    Refunded,
}

impl From<InespayPSyncStatus> for common_enums::AttemptStatus {
    fn from(item: InespayPSyncStatus) -> Self {
        match item {
            InespayPSyncStatus::Ok | InespayPSyncStatus::Settled => Self::Charged,
            InespayPSyncStatus::Created
            | InespayPSyncStatus::Opened
            | InespayPSyncStatus::BankSelected
            | InespayPSyncStatus::Initiated
            | InespayPSyncStatus::Pending
            | InespayPSyncStatus::Unfinished
            | InespayPSyncStatus::PartiallyAccepted => Self::AuthenticationPending,
            InespayPSyncStatus::Aborted
            | InespayPSyncStatus::Rejected
            | InespayPSyncStatus::Cancelled
            | InespayPSyncStatus::Failed => Self::Failure,
            InespayPSyncStatus::PartRefunded | InespayPSyncStatus::Refunded => Self::AutoRefunded,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InespayPSyncResponseData {
    cod_status: InespayPSyncStatus,
    status_desc: String,
    single_payin_id: String,
    single_payin_link: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum InespayPSyncResponse {
    InespayPSyncData(InespayPSyncResponseData),
    InespayPSyncWebhook(InespayPaymentWebhookData),
    InespayPSyncError(InespayErrorResponse),
}

impl<F, T> TryFrom<ResponseRouterData<F, InespayPSyncResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, InespayPSyncResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            InespayPSyncResponse::InespayPSyncData(data) => {
                let redirection_url = Url::parse(data.single_payin_link.as_str())
                    .change_context(errors::ConnectorError::ParsingFailed)?;
                let redirection_data = RedirectForm::from((redirection_url, Method::Get));

                Ok(Self {
                    status: common_enums::AttemptStatus::from(data.cod_status),
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(
                            data.single_payin_id.clone(),
                        ),
                        redirection_data: Box::new(Some(redirection_data)),
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
            InespayPSyncResponse::InespayPSyncWebhook(data) => {
                let status = enums::AttemptStatus::from(data.cod_status);
                Ok(Self {
                    status,
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(
                            data.single_payin_id.clone(),
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
            InespayPSyncResponse::InespayPSyncError(data) => Ok(Self {
                response: Err(ErrorResponse {
                    code: data.status.clone(),
                    message: data.status_desc.clone(),
                    reason: Some(data.status_desc.clone()),
                    attempt_status: None,
                    connector_transaction_id: None,
                    status_code: item.http_code,
                }),
                ..item.data
            }),
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InespayRefundRequest {
    single_payin_id: String,
    amount: Option<MinorUnit>,
}

impl<F> TryFrom<&InespayRouterData<&RefundsRouterData<F>>> for InespayRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &InespayRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let amount = utils::convert_back_amount_to_minor_units(
            &StringMinorUnitForConnector,
            item.amount.to_owned(),
            item.router_data.request.currency,
        )?;
        Ok(Self {
            single_payin_id: item.router_data.request.connector_transaction_id.clone(),
            amount: Some(amount),
        })
    }
}

#[derive(Debug, Serialize, Default, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InespayRSyncStatus {
    Confirmed,
    #[default]
    Pending,
    Rejected,
    Denied,
    Reversed,
    Mistake,
}

impl From<InespayRSyncStatus> for enums::RefundStatus {
    fn from(item: InespayRSyncStatus) -> Self {
        match item {
            InespayRSyncStatus::Confirmed => Self::Success,
            InespayRSyncStatus::Pending => Self::Pending,
            InespayRSyncStatus::Rejected
            | InespayRSyncStatus::Denied
            | InespayRSyncStatus::Reversed
            | InespayRSyncStatus::Mistake => Self::Failure,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundsData {
    status: String,
    status_desc: String,
    refund_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum InespayRefundsResponse {
    InespayRefundsData(RefundsData),
    InespayRefundsError(InespayErrorResponse),
}

impl TryFrom<RefundsResponseRouterData<Execute, InespayRefundsResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, InespayRefundsResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            InespayRefundsResponse::InespayRefundsData(data) => Ok(Self {
                response: Ok(RefundsResponseData {
                    connector_refund_id: data.refund_id,
                    refund_status: enums::RefundStatus::Pending,
                }),
                ..item.data
            }),
            InespayRefundsResponse::InespayRefundsError(data) => Ok(Self {
                response: Err(ErrorResponse {
                    code: data.status.clone(),
                    message: data.status_desc.clone(),
                    reason: Some(data.status_desc.clone()),
                    attempt_status: None,
                    connector_transaction_id: None,
                    status_code: item.http_code,
                }),
                ..item.data
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InespayRSyncResponseData {
    cod_status: InespayRSyncStatus,
    status_desc: String,
    refund_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum InespayRSyncResponse {
    InespayRSyncData(InespayRSyncResponseData),
    InespayRSyncWebhook(InespayRefundWebhookData),
    InespayRSyncError(InespayErrorResponse),
}

impl TryFrom<RefundsResponseRouterData<RSync, InespayRSyncResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, InespayRSyncResponse>,
    ) -> Result<Self, Self::Error> {
        let response = match item.response {
            InespayRSyncResponse::InespayRSyncData(data) => Ok(RefundsResponseData {
                connector_refund_id: data.refund_id,
                refund_status: enums::RefundStatus::from(data.cod_status),
            }),
            InespayRSyncResponse::InespayRSyncWebhook(data) => Ok(RefundsResponseData {
                connector_refund_id: data.refund_id,
                refund_status: enums::RefundStatus::from(data.cod_status),
            }),
            InespayRSyncResponse::InespayRSyncError(data) => Err(ErrorResponse {
                code: data.status.clone(),
                message: data.status_desc.clone(),
                reason: Some(data.status_desc.clone()),
                attempt_status: None,
                connector_transaction_id: None,
                status_code: item.http_code,
            }),
        };
        Ok(Self {
            response,
            ..item.data
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InespayPaymentWebhookData {
    pub single_payin_id: String,
    pub cod_status: InespayPSyncStatus,
    pub description: String,
    pub amount: MinorUnit,
    pub reference: String,
    pub creditor_account: Secret<String>,
    pub debtor_name: Secret<String>,
    pub debtor_account: Secret<String>,
    pub custom_data: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InespayRefundWebhookData {
    pub refund_id: String,
    pub simple_payin_id: String,
    pub cod_status: InespayRSyncStatus,
    pub description: String,
    pub amount: MinorUnit,
    pub reference: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum InespayWebhookEventData {
    Payment(InespayPaymentWebhookData),
    Refund(InespayRefundWebhookData),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InespayWebhookEvent {
    pub data_return: String,
    pub signature_data_return: String,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InespayErrorResponse {
    pub status: String,
    pub status_desc: String,
}

impl From<InespayWebhookEventData> for api_models::webhooks::IncomingWebhookEvent {
    fn from(item: InespayWebhookEventData) -> Self {
        match item {
            InespayWebhookEventData::Payment(payment_data) => match payment_data.cod_status {
                InespayPSyncStatus::Ok | InespayPSyncStatus::Settled => Self::PaymentIntentSuccess,
                InespayPSyncStatus::Failed | InespayPSyncStatus::Rejected => {
                    Self::PaymentIntentFailure
                }
                InespayPSyncStatus::Created
                | InespayPSyncStatus::Opened
                | InespayPSyncStatus::BankSelected
                | InespayPSyncStatus::Initiated
                | InespayPSyncStatus::Pending
                | InespayPSyncStatus::Unfinished
                | InespayPSyncStatus::PartiallyAccepted => Self::PaymentIntentProcessing,
                InespayPSyncStatus::Aborted
                | InespayPSyncStatus::Cancelled
                | InespayPSyncStatus::PartRefunded
                | InespayPSyncStatus::Refunded => Self::EventNotSupported,
            },
            InespayWebhookEventData::Refund(refund_data) => match refund_data.cod_status {
                InespayRSyncStatus::Confirmed => Self::RefundSuccess,
                InespayRSyncStatus::Rejected
                | InespayRSyncStatus::Denied
                | InespayRSyncStatus::Reversed
                | InespayRSyncStatus::Mistake => Self::RefundFailure,
                InespayRSyncStatus::Pending => Self::EventNotSupported,
            },
        }
    }
}
