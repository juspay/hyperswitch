use std::collections::HashMap;

use api_models::payments::PollConfig;
use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    pii::{self, Email, IpAddress},
    types::MinorUnit,
};
use hyperswitch_domain_models::{
    payment_method_data::{PaymentMethodData, UpiData},
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types,
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

use crate::{
    types::{
        CreateOrderResponseRouterData, PaymentsResponseRouterData, RefundsResponseRouterData,
        ResponseRouterData,
    },
    utils::{
        get_unimplemented_payment_method_error_message, missing_field_err,
        PaymentsAuthorizeRequestData, RouterData as OtherRouterData,
    },
};

pub struct RazorpayRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

impl<T> TryFrom<(MinorUnit, T)> for RazorpayRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from((amount, item): (MinorUnit, T)) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

pub const VERSION: i32 = 1;

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RazorpayOrderRequest {
    pub amount: MinorUnit,
    pub currency: enums::Currency,
    pub receipt: String,
    pub partial_payment: Option<bool>,
    pub first_payment_min_amount: Option<MinorUnit>,
    pub notes: Option<RazorpayNotes>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RazorpayNotes {
    Map(HashMap<String, String>),
    EmptyMap(HashMap<String, String>),
}

impl TryFrom<&RazorpayRouterData<&types::CreateOrderRouterData>> for RazorpayOrderRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &RazorpayRouterData<&types::CreateOrderRouterData>,
    ) -> Result<Self, Self::Error> {
        let currency = item.router_data.request.currency;
        let receipt = item.router_data.connector_request_reference_id.clone();

        Ok(Self {
            amount: item.amount,
            currency,
            receipt,
            partial_payment: None,
            first_payment_min_amount: None,
            notes: None,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RazorpayMetaData {
    pub order_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RazorpayOrderResponse {
    pub id: String,
}

impl TryFrom<CreateOrderResponseRouterData<RazorpayOrderResponse>>
    for types::CreateOrderRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: CreateOrderResponseRouterData<RazorpayOrderResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PaymentsResponseData::PaymentsCreateOrderResponse {
                order_id: item.response.id.clone(),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct UpiDetails {
    flow: UpiFlow,
    vpa: Secret<String, pii::UpiVpaMaskingStrategy>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpiFlow {
    Collect,
    Intent,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RazorpayPaymentsRequest {
    amount: MinorUnit,
    currency: enums::Currency,
    order_id: String,
    email: Email,
    contact: Secret<String>,
    method: RazorpayPaymentMethod,
    upi: UpiDetails,
    #[serde(skip_serializing_if = "Option::is_none")]
    ip: Option<Secret<String, IpAddress>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_agent: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RazorpayPaymentMethod {
    Upi,
}

impl TryFrom<&RazorpayRouterData<&types::PaymentsAuthorizeRouterData>> for RazorpayPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &RazorpayRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let payment_router_data = item.router_data;
        let router_request = &payment_router_data.request;
        let payment_method_data = &router_request.payment_method_data;

        let (razorpay_payment_method, upi_details) = match payment_method_data {
            PaymentMethodData::Upi(upi_type_data) => match upi_type_data {
                UpiData::UpiCollect(upi_collect_data) => {
                    let vpa_secret = upi_collect_data
                        .vpa_id
                        .clone()
                        .ok_or_else(missing_field_err("payment_method_data.upi.collect.vpa_id"))?;
                    (
                        RazorpayPaymentMethod::Upi,
                        UpiDetails {
                            flow: UpiFlow::Collect,
                            vpa: vpa_secret,
                        },
                    )
                }
                UpiData::UpiIntent(_) | UpiData::UpiQr(_) => {
                    Err(errors::ConnectorError::NotImplemented(
                        get_unimplemented_payment_method_error_message("razorpay"),
                    ))?
                }
            },
            _ => Err(errors::ConnectorError::NotImplemented(
                get_unimplemented_payment_method_error_message("razorpay"),
            ))?,
        };

        let contact_number = item.router_data.get_billing_phone_number()?;
        let order_id = router_request.get_order_id()?;
        let email = item.router_data.get_billing_email()?;
        let ip = router_request.get_ip_address_as_optional();
        let user_agent = router_request.get_optional_user_agent();

        Ok(Self {
            amount: item.amount,
            currency: router_request.currency,
            order_id,
            email,
            contact: contact_number,
            method: razorpay_payment_method,
            upi: upi_details,
            ip,
            user_agent,
        })
    }
}

pub struct RazorpayAuthType {
    pub(super) razorpay_id: Secret<String>,
    pub(super) razorpay_secret: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for RazorpayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                razorpay_id: api_key.to_owned(),
                razorpay_secret: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextAction {
    pub action: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RazorpayPaymentsResponse {
    pub razorpay_payment_id: String,
}

impl TryFrom<PaymentsResponseRouterData<RazorpayPaymentsResponse>>
    for types::PaymentsAuthorizeRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsResponseRouterData<RazorpayPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        let connector_metadata = get_wait_screen_metadata()?;
        let order_id = item.data.request.get_order_id()?;
        Ok(Self {
            status: enums::AttemptStatus::AuthenticationPending,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.razorpay_payment_id.clone(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id: Some(order_id),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitScreenData {
    display_from_timestamp: i128,
    display_to_timestamp: Option<i128>,
    poll_config: Option<PollConfig>,
}

pub fn get_wait_screen_metadata() -> CustomResult<Option<serde_json::Value>, errors::ConnectorError>
{
    let current_time = OffsetDateTime::now_utc().unix_timestamp_nanos();
    Ok(Some(serde_json::json!(WaitScreenData {
        display_from_timestamp: current_time,
        display_to_timestamp: Some(current_time + Duration::minutes(5).whole_nanoseconds()),
        poll_config: Some(PollConfig {
            delay_in_secs: 5,
            frequency: 5,
        }),
    })))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RazorpaySyncResponse {
    items: Vec<RazorpaySyncItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RazorpaySyncItem {
    id: String,
    status: RazorpayStatus,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RazorpayStatus {
    Created,
    Authorized,
    Captured,
    Refunded,
    Failed,
}

fn get_psync_razorpay_payment_status(razorpay_status: RazorpayStatus) -> enums::AttemptStatus {
    match razorpay_status {
        RazorpayStatus::Created => enums::AttemptStatus::Pending,
        RazorpayStatus::Authorized => enums::AttemptStatus::Authorized,
        RazorpayStatus::Captured | RazorpayStatus::Refunded => enums::AttemptStatus::Charged,
        RazorpayStatus::Failed => enums::AttemptStatus::Failure,
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, RazorpaySyncResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, RazorpaySyncResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = match item.response.items.last() {
            Some(last_item) => {
                let razorpay_status = last_item.status;
                get_psync_razorpay_payment_status(razorpay_status)
            }
            None => item.data.status,
        };
        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::NoResponseId,
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

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RazorpayRefundRequest {
    pub amount: MinorUnit,
}

impl<F> TryFrom<&RazorpayRouterData<&types::RefundsRouterData<F>>> for RazorpayRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &RazorpayRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RazorpayRefundResponse {
    pub id: String,
    pub status: RazorpayRefundStatus,
}

#[derive(Debug, Serialize, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RazorpayRefundStatus {
    Created,
    Processed,
    Failed,
    Pending,
}

impl From<RazorpayRefundStatus> for enums::RefundStatus {
    fn from(item: RazorpayRefundStatus) -> Self {
        match item {
            RazorpayRefundStatus::Processed => Self::Success,
            RazorpayRefundStatus::Pending | RazorpayRefundStatus::Created => Self::Pending,
            RazorpayRefundStatus::Failed => Self::Failure,
        }
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, RazorpayRefundResponse>>
    for types::RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RazorpayRefundResponse>,
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

// This code can be used later when Razorpay webhooks are implemented

// #[derive(Debug, Deserialize, Serialize)]
// #[serde(untagged)]
// pub enum RazorpayPaymentsResponseData {
//     PsyncResponse(RazorpaySyncResponse),
//     WebhookResponse(WebhookPaymentEntity),
// }

// impl From<RazorpayWebhookPaymentStatus> for enums::AttemptStatus {
//     fn from(status: RazorpayWebhookPaymentStatus) -> Self {
//         match status {
//             RazorpayWebhookPaymentStatus::Authorized => Self::Authorized,
//             RazorpayWebhookPaymentStatus::Captured => Self::Charged,
//             RazorpayWebhookPaymentStatus::Failed => Self::Failure,
//         }
//     }
// }

// impl<F, T> TryFrom<ResponseRouterData<F, RazorpayPaymentsResponseData, T, PaymentsResponseData>>
//     for RouterData<F, T, PaymentsResponseData>
// {
//     type Error = error_stack::Report<errors::ConnectorError>;
//     fn try_from(
//         item: ResponseRouterData<F, RazorpayPaymentsResponseData, T, PaymentsResponseData>,
//     ) -> Result<Self, Self::Error> {
//         match item.response {
//             RazorpayPaymentsResponseData::PsyncResponse(sync_response) => {
//                 let status = get_psync_razorpay_payment_status(sync_response.status.clone());
//                 Ok(Self {
//                     status,
//                     response: if is_payment_failure(status) {
//                         Err(RouterErrorResponse {
//                             code: sync_response.status.clone().to_string(),
//                             message: sync_response.status.clone().to_string(),
//                             reason: Some(sync_response.status.to_string()),
//                             status_code: item.http_code,
//                             attempt_status: Some(status),
//                             connector_transaction_id: None,
//                             network_advice_code: None,
//                             network_decline_code: None,
//                             network_error_message: None,
//                         })
//                     } else {
//                         Ok(PaymentsResponseData::TransactionResponse {
//                             resource_id: ResponseId::NoResponseId,
//                             redirection_data: Box::new(None),
//                             mandate_reference: Box::new(None),
//                             connector_metadata: None,
//                             network_txn_id: None,
//                             connector_response_reference_id: None,
//                             incremental_authorization_allowed: None,
//                             charges: None,
//                         })
//                     },
//                     ..item.data
//                 })
//             }
//             RazorpayPaymentsResponseData::WebhookResponse(webhook_payment_entity) => {
//                 let razorpay_status = webhook_payment_entity.status;
//                 let status = enums::AttemptStatus::from(razorpay_status.clone());

//                 Ok(Self {
//                     status,
//                     response: if is_payment_failure(status) {
//                         Err(RouterErrorResponse {
//                             code: razorpay_status.clone().to_string(),
//                             message: razorpay_status.clone().to_string(),
//                             reason: Some(razorpay_status.to_string()),
//                             status_code: item.http_code,
//                             attempt_status: Some(status),
//                             connector_transaction_id: Some(webhook_payment_entity.id.clone()),
//                             network_advice_code: None,
//                             network_decline_code: None,
//                             network_error_message: None,
//                         })
//                     } else {
//                         Ok(PaymentsResponseData::TransactionResponse {
//                             resource_id: ResponseId::ConnectorTransactionId(
//                                 webhook_payment_entity.id.clone(),
//                             ),
//                             redirection_data: Box::new(None),
//                             mandate_reference: Box::new(None),
//                             connector_metadata: None,
//                             network_txn_id: None,
//                             connector_response_reference_id: Some(webhook_payment_entity.id),
//                             incremental_authorization_allowed: None,
//                             charges: None,
//                         })
//                     },
//                     ..item.data
//                 })
//             }
//         }
//     }
// }

impl TryFrom<RefundsResponseRouterData<RSync, RazorpayRefundResponse>>
    for types::RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RazorpayRefundResponse>,
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ErrorResponse {
    RazorpayErrorResponse(RazorpayErrorResponse),
    RazorpayStringError(String),
    RazorpayError(RazorpayErrorMessage),
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RazorpayErrorMessage {
    pub message: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RazorpayErrorResponse {
    pub error: RazorpayError,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RazorpayError {
    pub code: String,
    pub description: String,
    pub source: Option<String>,
    pub step: Option<String>,
    pub reason: Option<String>,
    pub metadata: Option<Metadata>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Metadata {
    pub order_id: Option<String>,
}

// This code can be used later when Razorpay webhooks are implemented

// #[derive(Debug, Serialize, Deserialize)]

// pub struct RazorpayWebhookPayload {
//     pub event: RazorpayWebhookEventType,
//     pub payload: RazorpayWebhookPayloadBody,
// }

// #[derive(Debug, Serialize, Deserialize)]
// #[serde(untagged)]
// pub enum RazorpayWebhookEventType {
//     Payments(RazorpayWebhookPaymentEvent),
//     Refunds(RazorpayWebhookRefundEvent),
// }

// #[derive(Debug, Serialize, Deserialize)]
// pub struct RazorpayWebhookPayloadBody {
//     pub refund: Option<RazorpayRefundWebhookPayload>,
//     pub payment: RazorpayPaymentWebhookPayload,
// }

// #[derive(Debug, Serialize, Deserialize)]
// pub struct RazorpayPaymentWebhookPayload {
//     pub entity: WebhookPaymentEntity,
// }

// #[derive(Debug, Serialize, Deserialize)]
// pub struct RazorpayRefundWebhookPayload {
//     pub entity: WebhookRefundEntity,
// }

// #[derive(Debug, Serialize, Deserialize)]
// pub struct WebhookRefundEntity {
//     pub id: String,
//     pub status: RazorpayWebhookRefundEvent,
// }

// #[derive(Debug, Serialize, Eq, PartialEq, Deserialize)]
// pub enum RazorpayWebhookRefundEvent {
//     #[serde(rename = "refund.created")]
//     Created,
//     #[serde(rename = "refund.processed")]
//     Processed,
//     #[serde(rename = "refund.failed")]
//     Failed,
//     #[serde(rename = "refund.speed_change")]
//     SpeedChange,
// }

// #[derive(Debug, Serialize, Deserialize)]
// pub struct WebhookPaymentEntity {
//     pub id: String,
//     pub status: RazorpayWebhookPaymentStatus,
// }

// #[derive(Debug, Serialize, Eq, PartialEq, Clone, Deserialize)]
// #[serde(rename_all = "snake_case")]
// pub enum RazorpayWebhookPaymentStatus {
//     Authorized,
//     Captured,
//     Failed,
// }

// #[derive(Debug, Serialize, Eq, PartialEq, Deserialize)]
// pub enum RazorpayWebhookPaymentEvent {
//     #[serde(rename = "payment.authorized")]
//     Authorized,
//     #[serde(rename = "payment.captured")]
//     Captured,
//     #[serde(rename = "payment.failed")]
//     Failed,
// }

// impl TryFrom<RazorpayWebhookEventType> for api_models::webhooks::IncomingWebhookEvent {
//     type Error = errors::ConnectorError;

//     fn try_from(event_type: RazorpayWebhookEventType) -> Result<Self, Self::Error> {
//         match event_type {
//             RazorpayWebhookEventType::Payments(payment_event) => match payment_event {
//                 RazorpayWebhookPaymentEvent::Authorized => {
//                     Ok(Self::PaymentIntentAuthorizationSuccess)
//                 }
//                 RazorpayWebhookPaymentEvent::Captured => Ok(Self::PaymentIntentSuccess),
//                 RazorpayWebhookPaymentEvent::Failed => Ok(Self::PaymentIntentFailure),
//             },
//             RazorpayWebhookEventType::Refunds(refund_event) => match refund_event {
//                 RazorpayWebhookRefundEvent::Processed => Ok(Self::RefundSuccess),
//                 RazorpayWebhookRefundEvent::Created => Ok(Self::RefundSuccess),
//                 RazorpayWebhookRefundEvent::Failed => Ok(Self::RefundFailure),
//                 RazorpayWebhookRefundEvent::SpeedChange => Ok(Self::EventNotSupported),
//             },
//         }
//     }
// }
