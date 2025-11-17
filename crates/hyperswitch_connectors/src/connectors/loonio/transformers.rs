use std::collections::HashMap;

#[cfg(feature = "payouts")]
use api_models::payouts::{BankRedirect, PayoutMethodData};
use api_models::webhooks;
use common_enums::{enums, Currency};
use common_utils::{
    id_type,
    pii::{self, Email},
    request::Method,
    types::FloatMajorUnit,
};
use hyperswitch_domain_models::{
    payment_method_data::{BankRedirectData, PaymentMethodData},
    router_data::{
        AdditionalPaymentMethodConnectorResponse, ConnectorAuthType, ConnectorResponseData,
        InteracCustomerInfo, RouterData,
    },
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::{
    router_flow_types::PoFulfill, router_response_types::PayoutsResponseData,
    types::PayoutsRouterData,
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

#[cfg(feature = "payouts")]
use crate::types::PayoutsResponseRouterData;
use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self, PaymentsAuthorizeRequestData, RouterData as _},
};
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
                Ok(Self {
                    currency_code: item.router_data.request.currency,
                    customer_profile,
                    amount: item.amount,
                    customer_id: item.router_data.get_customer_id()?,
                    transaction_id,
                    payment_method_type: InteracPaymentMethodType::InteracEtransfer,
                    redirect_url: Some(redirect_url),
                    webhook_url: Some(item.router_data.request.get_webhook_url()?),
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
    pub customer_bank_info: Option<pii::SecretSerdeValue>,
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
            LoonioPaymentResponseData::Sync(sync_response) => {
                let connector_response =
                    sync_response
                        .customer_bank_info
                        .as_ref()
                        .map(|customer_info| {
                            ConnectorResponseData::with_additional_payment_method_data(
                                AdditionalPaymentMethodConnectorResponse::BankRedirect {
                                    interac: Some(InteracCustomerInfo {
                                        customer_info: Some(customer_info.clone()),
                                    }),
                                },
                            )
                        });
                Ok(Self {
                    status: enums::AttemptStatus::from(sync_response.state),
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(
                            sync_response.transaction_id,
                        ),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        incremental_authorization_allowed: None,
                        charges: None,
                    }),
                    connector_response,
                    ..item.data
                })
            }
            LoonioPaymentResponseData::Webhook(webhook_body) => {
                let payment_status = enums::AttemptStatus::from(&webhook_body.event_code);
                let connector_response = webhook_body.customer_info.as_ref().map(|customer_info| {
                    ConnectorResponseData::with_additional_payment_method_data(
                        AdditionalPaymentMethodConnectorResponse::BankRedirect {
                            interac: Some(InteracCustomerInfo {
                                customer_info: Some(customer_info.clone()),
                            }),
                        },
                    )
                });
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
                    connector_response,
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
    #[serde(rename = "type")]
    pub transaction_type: LoonioWebhookTransactionType,
    pub customer_info: Option<pii::SecretSerdeValue>,
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

pub(crate) fn get_loonio_webhook_event(
    transaction_type: &LoonioWebhookTransactionType,
    event_code: &LoonioWebhookEventCode,
) -> webhooks::IncomingWebhookEvent {
    match transaction_type {
        LoonioWebhookTransactionType::OutgoingNotVerified => {
            #[cfg(feature = "payouts")]
            {
                match event_code {
                    LoonioWebhookEventCode::TransactionPrepared => {
                        webhooks::IncomingWebhookEvent::PayoutCreated
                    }
                    LoonioWebhookEventCode::TransactionPending => {
                        webhooks::IncomingWebhookEvent::PayoutProcessing
                    }
                    LoonioWebhookEventCode::TransactionAvailable
                    | LoonioWebhookEventCode::TransactionSettled => {
                        webhooks::IncomingWebhookEvent::PayoutSuccess
                    }
                    LoonioWebhookEventCode::TransactionFailed
                    | LoonioWebhookEventCode::TransactionRejected => {
                        webhooks::IncomingWebhookEvent::PayoutFailure
                    }
                    _ => webhooks::IncomingWebhookEvent::EventNotSupported,
                }
            }

            #[cfg(not(feature = "payouts"))]
            {
                webhooks::IncomingWebhookEvent::EventNotSupported
            }
        }

        _ => match event_code {
            LoonioWebhookEventCode::TransactionSettled
            | LoonioWebhookEventCode::TransactionAvailable => {
                webhooks::IncomingWebhookEvent::PaymentIntentSuccess
            }
            LoonioWebhookEventCode::TransactionPending
            | LoonioWebhookEventCode::TransactionPrepared => {
                webhooks::IncomingWebhookEvent::PaymentIntentProcessing
            }
            LoonioWebhookEventCode::TransactionFailed
            | LoonioWebhookEventCode::TransactionRejected
            | LoonioWebhookEventCode::TransactionStatusFileFailed
            | LoonioWebhookEventCode::TransactionReturned
            | LoonioWebhookEventCode::TransactionWrongDestination
            | LoonioWebhookEventCode::TransactionNsf => {
                webhooks::IncomingWebhookEvent::PaymentIntentFailure
            }
            _ => webhooks::IncomingWebhookEvent::EventNotSupported,
        },
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

// Payout Structures
#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
pub struct LoonioPayoutFulfillRequest {
    pub currency_code: Currency,
    pub customer_profile: LoonioCustomerProfile,
    pub amount: FloatMajorUnit,
    pub customer_id: id_type::CustomerId,
    pub transaction_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook_url: Option<String>,
}

#[cfg(feature = "payouts")]
impl TryFrom<&LoonioRouterData<&PayoutsRouterData<PoFulfill>>> for LoonioPayoutFulfillRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &LoonioRouterData<&PayoutsRouterData<PoFulfill>>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.get_payout_method_data()? {
            PayoutMethodData::BankRedirect(BankRedirect::Interac(interac_data)) => {
                let customer_profile = LoonioCustomerProfile {
                    first_name: item.router_data.get_billing_first_name()?,
                    last_name: item.router_data.get_billing_last_name()?,
                    email: interac_data.email,
                };

                Ok(Self {
                    currency_code: item.router_data.request.destination_currency,
                    customer_profile,
                    amount: item.amount,
                    customer_id: item.router_data.get_customer_id()?,
                    transaction_id: item.router_data.connector_request_reference_id.clone(),
                    webhook_url: item.router_data.request.webhook_url.clone(),
                })
            }
            PayoutMethodData::Card(_)
            | PayoutMethodData::Bank(_)
            | PayoutMethodData::Wallet(_)
            | PayoutMethodData::Passthrough(_) => Err(errors::ConnectorError::NotSupported {
                message: "Payment Method Not Supported".to_string(),
                connector: "Loonio",
            })?,
        }
    }
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoonioPayoutFulfillResponse {
    pub id: i64,
    pub api_transaction_id: String,
    #[serde(rename = "type")]
    pub transaction_type: String,
    pub state: LoonioPayoutStatus,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LoonioPayoutStatus {
    Created,
    Prepared,
    Pending,
    Settled,
    Available,
    Rejected,
    Abandoned,
    ConnectedAbandoned,
    ConnectedInsufficientFunds,
    Failed,
    Nsf,
    Returned,
    Rollback,
}

#[cfg(feature = "payouts")]
impl From<LoonioPayoutStatus> for enums::PayoutStatus {
    fn from(item: LoonioPayoutStatus) -> Self {
        match item {
            LoonioPayoutStatus::Created | LoonioPayoutStatus::Prepared => Self::Initiated,
            LoonioPayoutStatus::Pending => Self::Pending,
            LoonioPayoutStatus::Settled | LoonioPayoutStatus::Available => Self::Success,
            LoonioPayoutStatus::Rejected
            | LoonioPayoutStatus::Abandoned
            | LoonioPayoutStatus::ConnectedAbandoned
            | LoonioPayoutStatus::ConnectedInsufficientFunds
            | LoonioPayoutStatus::Failed
            | LoonioPayoutStatus::Nsf
            | LoonioPayoutStatus::Returned
            | LoonioPayoutStatus::Rollback => Self::Failed,
        }
    }
}

#[cfg(feature = "payouts")]
impl<F> TryFrom<PayoutsResponseRouterData<F, LoonioPayoutFulfillResponse>>
    for PayoutsRouterData<F>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PayoutsResponseRouterData<F, LoonioPayoutFulfillResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PayoutsResponseData {
                status: Some(enums::PayoutStatus::from(item.response.state)),
                connector_payout_id: Some(item.response.api_transaction_id),
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
                error_code: None,
                error_message: None,
                payout_connector_metadata: None,
            }),
            ..item.data
        })
    }
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoonioPayoutSyncResponse {
    pub transaction_id: String,
    pub state: LoonioPayoutStatus,
}

#[cfg(feature = "payouts")]
impl<F> TryFrom<PayoutsResponseRouterData<F, LoonioPayoutSyncResponse>> for PayoutsRouterData<F> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PayoutsResponseRouterData<F, LoonioPayoutSyncResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PayoutsResponseData {
                status: Some(enums::PayoutStatus::from(item.response.state)),
                connector_payout_id: Some(item.response.transaction_id.to_string()),
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
                error_code: None,
                error_message: None,
                payout_connector_metadata: None,
            }),
            ..item.data
        })
    }
}
