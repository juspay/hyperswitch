use std::collections::HashMap;

use api_models::webhooks;
use common_enums::{enums, Currency};
use common_utils::{id_type, pii::Email, request::Method, types::FloatMajorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{BankRedirectData, PaymentMethodData},
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self, PaymentsAuthorizeRequestData, RouterData as _},
};
use common_utils::crypto::{HmacSha256, SignMessage};
pub struct LoonioRouterData<T> {
    pub amount: FloatMajorUnit,
    pub router_data: T,
}

impl<T> From<(FloatMajorUnit, T)> for LoonioRouterData<T> {
    fn from((amount, item): (FloatMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

// Auth Struct
pub struct LoonioAuthType {
    pub(super) merchant_id: Secret<String>,
    pub(super) merchant_token: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for LoonioAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                merchant_id: api_key.to_owned(),
                merchant_token: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct LoonioPaymentRequest {
    pub currency_code: Currency,
    pub customer_profile: LoonioCustomerProfile,
    pub amount: FloatMajorUnit,
    pub customer_id: id_type::CustomerId,
    pub transaction_id: String,
    pub payment_method_type: InteracPaymentMethodType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_url: Option<LoonioRedirectUrl>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook_url: Option<String>,
    pub signature: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InteracPaymentMethodType {
    InteracEtransfer,
}

#[derive(Debug, Serialize)]
pub struct LoonioCustomerProfile {
    pub first_name: Secret<String>,
    pub last_name: Secret<String>,
    pub email: Email,
}

#[derive(Debug, Serialize)]
pub struct LoonioRedirectUrl {
    pub success_url: String,
    pub failed_url: String,
}

impl TryFrom<&LoonioRouterData<&PaymentsAuthorizeRouterData>> for LoonioPaymentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &LoonioRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth = LoonioAuthType::try_from(&item.router_data.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::BankRedirect(BankRedirectData::Interac { .. }) => {
                let transaction_id = item.router_data.connector_request_reference_id.clone();

                let customer_profile = LoonioCustomerProfile {
                    first_name: item.router_data.get_billing_first_name()?,
                    last_name: item.router_data.get_billing_last_name()?,
                    email: item.router_data.get_billing_email()?,
                };

                let redirect_url = LoonioRedirectUrl {
                    success_url: item.router_data.request.get_router_return_url()?,
                    failed_url: item.router_data.request.get_router_return_url()?,
                };
                // Generate HMAC-SHA256 signature using transaction_id and merchant_token

                let signature = HmacSha256::sign_message(
                    &HmacSha256,
                    auth.merchant_token.expose().as_bytes(),
                    transaction_id.as_bytes(),
                )
                .change_context(errors::ConnectorError::RequestEncodingFailed)
                .map(hex::encode)?;

                Ok(Self {
                    currency_code: item.router_data.request.currency,
                    customer_profile,
                    amount: item.amount,
                    customer_id: item.router_data.get_customer_id()?,
                    transaction_id,
                    payment_method_type: InteracPaymentMethodType::InteracEtransfer,
                    redirect_url: Some(redirect_url),
                    webhook_url: Some(item.router_data.request.get_webhook_url()?),
                    signature: Some(signature),
                })
            }
            PaymentMethodData::BankRedirect(_) => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Loonio"),
            ))?,

            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Loonio"),
            )
            .into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoonioPaymentsResponse {
    pub payment_form: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, LoonioPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, LoonioPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::AuthenticationPending,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.data.connector_request_reference_id.clone(),
                ),
                redirection_data: Box::new(Some(RedirectForm::Form {
                    endpoint: item.response.payment_form,
                    method: Method::Get,
                    form_fields: HashMap::new(),
                })),
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LoonioTransactionStatus {
    Created,
    Prepared,
    Pending,
    Settled,
    Available,
    Abandoned,
    Rejected,
    Failed,
    Rollback,
    Returned,
    Nsf,
}

impl From<LoonioTransactionStatus> for enums::AttemptStatus {
    fn from(item: LoonioTransactionStatus) -> Self {
        match item {
            LoonioTransactionStatus::Created => Self::AuthenticationPending,
            LoonioTransactionStatus::Prepared | LoonioTransactionStatus::Pending => Self::Pending,
            LoonioTransactionStatus::Settled | LoonioTransactionStatus::Available => Self::Charged,
            LoonioTransactionStatus::Abandoned
            | LoonioTransactionStatus::Rejected
            | LoonioTransactionStatus::Failed
            | LoonioTransactionStatus::Returned
            | LoonioTransactionStatus::Nsf => Self::Failure,
            LoonioTransactionStatus::Rollback => Self::Voided,
        }
    }
}

// Sync Response Structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoonioTransactionSyncResponse {
    pub transaction_id: String,
    pub state: LoonioTransactionStatus,
}

#[derive(Default, Debug, Serialize)]
pub struct LoonioRefundRequest {
    pub amount: FloatMajorUnit,
}

impl<F> TryFrom<&LoonioRouterData<&RefundsRouterData<F>>> for LoonioRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &LoonioRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, LoonioPaymentResponseData, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, LoonioPaymentResponseData, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            LoonioPaymentResponseData::Sync(sync_response) => Ok(Self {
                status: enums::AttemptStatus::from(sync_response.state),
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(sync_response.transaction_id),
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
            LoonioPaymentResponseData::Webhook(webhook_body) => {
                let payment_status = enums::AttemptStatus::from(&webhook_body.event_code);
                Ok(Self {
                    status: payment_status,
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(
                            webhook_body.api_transaction_id,
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
        }
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

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct LoonioErrorResponse {
    pub status: u16,
    pub error_code: Option<String>,
    pub message: String,
}

// Webhook related structs

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LoonioWebhookEventCode {
    TransactionPrepared,
    TransactionPending,
    TransactionAvailable,
    TransactionSettled,
    TransactionFailed,
    TransactionRejected,
    #[serde(rename = "TRANSACTION_WAITING_STATUS_FILE")]
    TransactionWaitingStatusFile,
    #[serde(rename = "TRANSACTION_STATUS_FILE_RECEIVED")]
    TransactionStatusFileReceived,
    #[serde(rename = "TRANSACTION_STATUS_FILE_FAILED")]
    TransactionStatusFileFailed,
    #[serde(rename = "TRANSACTION_RETURNED")]
    TransactionReturned,
    #[serde(rename = "TRANSACTION_WRONG_DESTINATION")]
    TransactionWrongDestination,
    #[serde(rename = "TRANSACTION_NSF")]
    TransactionNsf,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LoonioWebhookTransactionType {
    Incoming,
    OutgoingVerified,
    OutgoingNotVerified,
    OutgoingCustomerDefined,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoonioWebhookBody {
    pub amount: FloatMajorUnit,
    pub api_transaction_id: String,
    pub signature: Option<String>,
    pub event_code: LoonioWebhookEventCode,
    pub id: i32,
    #[serde(rename = "type")]
    pub transaction_type: LoonioWebhookTransactionType,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LoonioPaymentResponseData {
    Sync(LoonioTransactionSyncResponse),
    Webhook(LoonioWebhookBody),
}
impl From<&LoonioWebhookEventCode> for webhooks::IncomingWebhookEvent {
    fn from(event_code: &LoonioWebhookEventCode) -> Self {
        match event_code {
            LoonioWebhookEventCode::TransactionSettled
            | LoonioWebhookEventCode::TransactionAvailable => Self::PaymentIntentSuccess,
            LoonioWebhookEventCode::TransactionPending
            | LoonioWebhookEventCode::TransactionPrepared => Self::PaymentIntentProcessing,
            LoonioWebhookEventCode::TransactionFailed
            // deprecated
            | LoonioWebhookEventCode::TransactionRejected
            | LoonioWebhookEventCode::TransactionStatusFileFailed
            | LoonioWebhookEventCode::TransactionReturned
            | LoonioWebhookEventCode::TransactionWrongDestination
            | LoonioWebhookEventCode::TransactionNsf => Self::PaymentIntentFailure,
            _ => Self::EventNotSupported,
        }
    }
}

impl From<&LoonioWebhookEventCode> for enums::AttemptStatus {
    fn from(event_code: &LoonioWebhookEventCode) -> Self {
        match event_code {
            LoonioWebhookEventCode::TransactionSettled
            | LoonioWebhookEventCode::TransactionAvailable => Self::Charged,

            LoonioWebhookEventCode::TransactionPending
            | LoonioWebhookEventCode::TransactionPrepared => Self::Pending,

            LoonioWebhookEventCode::TransactionFailed
            | LoonioWebhookEventCode::TransactionRejected
            | LoonioWebhookEventCode::TransactionStatusFileFailed
            | LoonioWebhookEventCode::TransactionReturned
            | LoonioWebhookEventCode::TransactionWrongDestination
            | LoonioWebhookEventCode::TransactionNsf => Self::Failure,

            _ => Self::Pending,
        }
    }
}
