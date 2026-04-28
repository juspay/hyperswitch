use hyperswitch_domain_models::{
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_request_types::ResponseId,
    router_response_types::PaymentsResponseData,
};
use hyperswitch_interfaces::{
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    errors,
};
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{types::ResponseRouterData, utils};

pub struct SanlamAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) merchant_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for SanlamAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_owned(),
                merchant_id: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum SanlamWebhookEvent {
    Payment(SanlamPaymentWebhookEvent),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SanlamPaymentWebhookEvent {
    pub event_type: SanlamWebhookEventType,
    pub payment: SanlamWebhookPayment,
    pub error: Option<SanlamWebhookError>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SanlamWebhookError {
    pub code: Option<String>,
    pub message: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum SanlamWebhookEventType {
    #[serde(rename = "payment.succeeded")]
    PaymentSucceeded,
    #[serde(rename = "payment.failed")]
    PaymentFailed,
    #[serde(rename = "dispute.opened")]
    DisputeOpened,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SanlamWebhookPayment {
    pub user_reference: String,
    pub status: SanlamPaymentStatus,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SanlamPaymentStatus {
    Success,
    Failure,
    Dispute,
}

impl<F, T> TryFrom<ResponseRouterData<F, SanlamWebhookEvent, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, SanlamWebhookEvent, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            SanlamWebhookEvent::Payment(payment_event) => {
                let status = common_enums::AttemptStatus::try_from(&payment_event.payment.status)?;
                let response = if utils::is_payment_failure(status) {
                    Err(ErrorResponse {
                        code: payment_event
                            .error
                            .as_ref()
                            .and_then(|e| e.code.clone())
                            .unwrap_or(NO_ERROR_CODE.to_string()),
                        message: payment_event
                            .error
                            .as_ref()
                            .and_then(|e| e.message.clone())
                            .unwrap_or(NO_ERROR_MESSAGE.to_string()),
                        reason: payment_event.error.as_ref().and_then(|e| e.reason.clone()),
                        attempt_status: None,
                        connector_transaction_id: None,
                        connector_response_reference_id: None,
                        status_code: item.http_code,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    })
                } else {
                    Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::NoResponseId,
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        incremental_authorization_allowed: None,
                        authentication_data: None,
                        charges: None,
                    })
                };

                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            }
        }
    }
}

impl TryFrom<&SanlamPaymentStatus> for common_enums::AttemptStatus {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &SanlamPaymentStatus) -> Result<Self, Self::Error> {
        match item {
            SanlamPaymentStatus::Success => Ok(Self::Charged),
            SanlamPaymentStatus::Failure => Ok(Self::Failure),
            SanlamPaymentStatus::Dispute => {
                Err(errors::ConnectorError::ResponseDeserializationFailed)?
            }
        }
    }
}
