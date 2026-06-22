use hyperswitch_domain_models::{router_request_types::*, router_response_types::*};
use hyperswitch_interfaces::errors::ConnectorError;
use hyperswitch_masking::{ExposeInterface as _, Secret};
use serde::{Deserialize, Serialize};

use crate::utils::ForeignTryFrom;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldpaymodularPaymentsResponse {
    pub outcome: PaymentOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_factors: Option<Secret<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer: Option<Secret<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheme: Option<Secret<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_instrument: Option<Secret<serde_json::Value>>,
    #[serde(rename = "_links")]
    pub links: PaymentLinks,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldpayModularRefundResponse {
    pub payment_id: String,
    #[serde(rename = "_links")]
    pub links: PaymentLinks,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PaymentOutcome {
    #[serde(alias = "authorized", alias = "Authorized")]
    Authorized,
    Refused,
    #[serde(alias = "Sent for Settlement")]
    SentForSettlement,
    #[serde(alias = "Sent for Refund")]
    SentForRefund,
    #[serde(alias = "Sent for Cancellation")]
    SentForCancellation,
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldpaymodularEventResponse {
    pub last_event: EventType,
    #[serde(rename = "_links", skip_serializing_if = "Option::is_none")]
    pub links: Option<Secret<serde_json::Value>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WroldpayModularActualPsyncResponseObj {
    pub last_event: PaymentOutcome,
    pub value: super::PaymentValue,
}

// Sent for settlement plays totally differnt role in worldpaymodular in webhooks and psync
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WorldpayModularPsyncObjResponse {
    PsyncResponse(WroldpayModularActualPsyncResponseObj),
    Webhook(WorldpaymodularWebhookEventType),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldpaymodularCaptureResponse {
    pub payment_id: String,
    #[serde(rename = "_links")]
    pub links: PaymentLinks,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldpaymodularVoidResponse {
    #[serde(rename = "_links", skip_serializing_if = "Option::is_none")]
    pub links: Option<PaymentLinks>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EventType {
    #[serde(alias = "sentForAuthorization", alias = "Sent for Authorization")]
    SentForAuthorization,
    #[serde(alias = "Authorized", alias = "authorized")]
    Authorized,
    #[serde(alias = "Sent for Settlement", alias = "sentForSettlement")]
    SentForSettlement,
    #[serde(alias = "Settlement Failed", alias = "settlementFailed")]
    SettlementFailed,
    #[serde(alias = "Settlement Rejected", alias = "settlementRejected")]
    SettlementRejected,
    Cancelled,
    Error,
    Expired,
    Refused,
    #[serde(alias = "Sent for Refund", alias = "sentForRefund")]
    SentForRefund,
    RefundFailed,
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct PaymentLinks {
    #[serde(
        rename = "cardPayments:events",
        skip_serializing_if = "Option::is_none"
    )]
    pub events: Option<PaymentLink>,
    #[serde(
        rename = "cardPayments:settle",
        skip_serializing_if = "Option::is_none"
    )]
    pub settle_event: Option<PaymentLink>,
    #[serde(
        rename = "cardPayments:partialSettle",
        skip_serializing_if = "Option::is_none"
    )]
    pub partial_settle_event: Option<PaymentLink>,
    #[serde(
        rename = "cardPayments:refund",
        skip_serializing_if = "Option::is_none"
    )]
    pub refund_event: Option<PaymentLink>,
    #[serde(
        rename = "cardPayments:partialRefund",
        skip_serializing_if = "Option::is_none"
    )]
    pub partial_refund_event: Option<PaymentLink>,
    #[serde(
        rename = "cardPayments:reverse",
        skip_serializing_if = "Option::is_none"
    )]
    pub reverse_event: Option<PaymentLink>,
    #[serde(rename = "tokens:token", skip_serializing_if = "Option::is_none")]
    pub token: Option<SecretPaymentLink>,
}

impl PaymentLinks {
    pub fn get_resource_id(&self) -> Result<ResponseId, error_stack::Report<ConnectorError>> {
        self.events
            .clone()
            .ok_or(
                ConnectorError::MissingRequiredField {
                    field_name: "href resource id",
                }
                .into(),
            )
            .and_then(|event| event.get_response_id())
    }
    pub fn get_response_id_str(
        &self,
    ) -> Result<ResponseIdStr, error_stack::Report<ConnectorError>> {
        self.events
            .clone()
            .ok_or(
                ConnectorError::MissingRequiredField {
                    field_name: "href resource id",
                }
                .into(),
            )
            .and_then(|event| event.get_response_id_str())
    }
    pub fn get_mandate_id(&self) -> Option<MandateReference> {
        self.token.clone().map(|mandate_id| MandateReference {
            connector_mandate_id: Some(mandate_id.href.expose()),
            payment_method_id: None,
            mandate_metadata: None,
            connector_mandate_request_reference_id: None,
        })
    }
}

impl SecretPaymentLink {
    pub fn get_event_data(&self) -> Option<String> {
        self.href
            .clone()
            .expose()
            .rsplit_once('/')
            .map(|h| h.1.to_string())
    }
}
impl PaymentLink {
    pub fn get_event_data(&self) -> Option<String> {
        self.href.rsplit_once('/').map(|h| h.1.to_string())
    }

    pub fn get_response_id_str(
        &self,
    ) -> Result<ResponseIdStr, error_stack::Report<ConnectorError>> {
        let id = self.href.rsplit_once('/').map(|h| h.1.to_string()).ok_or(
            ConnectorError::MissingRequiredField {
                field_name: "href resource id",
            },
        )?;
        Ok(ResponseIdStr { id })
    }

    pub fn get_response_id(&self) -> Result<ResponseId, error_stack::Report<ConnectorError>> {
        let id = self.href.rsplit_once('/').map(|h| h.1.to_string()).ok_or(
            ConnectorError::MissingRequiredField {
                field_name: "href resource id",
            },
        )?;
        Ok(ResponseId::ConnectorTransactionId(id))
    }
}
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct PaymentLink {
    pub href: String,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct SecretPaymentLink {
    pub href: Secret<String>,
}

fn get_resource_id<T, F>(
    links: Option<PaymentLinks>,
    transform_fn: F,
) -> Result<T, error_stack::Report<ConnectorError>>
where
    F: Fn(String) -> T,
{
    let reference_id = links
        .and_then(|l| l.events)
        .and_then(|e| e.href.rsplit_once('/').map(|h| h.1.to_string()))
        .map(transform_fn);
    reference_id.ok_or_else(|| {
        ConnectorError::MissingRequiredField {
            field_name: "links.events",
        }
        .into()
    })
}

pub struct ResponseIdStr {
    pub id: String,
}

impl TryFrom<Option<PaymentLinks>> for ResponseIdStr {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(links: Option<PaymentLinks>) -> Result<Self, Self::Error> {
        get_resource_id(links, |id| Self { id })
    }
}

impl ForeignTryFrom<Option<PaymentLinks>> for ResponseId {
    type Error = error_stack::Report<ConnectorError>;
    fn foreign_try_from(links: Option<PaymentLinks>) -> Result<Self, Self::Error> {
        get_resource_id(links, Self::ConnectorTransactionId)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorldpaymodularErrorResponse {
    pub error_name: String,
    pub message: String,
    pub validation_errors: Option<serde_json::Value>,
}

impl WorldpaymodularErrorResponse {
    pub fn default(status_code: u16) -> Self {
        match status_code {
            code @ 404 => Self {
                error_name: format!("{} Not found", code),
                message: "Resource not found".to_string(),
                validation_errors: None,
            },
            code => Self {
                error_name: code.to_string(),
                message: "Unknown error".to_string(),
                validation_errors: None,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldpaymodularWebhookTransactionId {
    pub event_details: EventDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventDetails {
    pub transaction_reference: String,
    #[serde(rename = "type")]
    pub event_type: EventType,
    pub reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldpaymodularWebhookEventType {
    pub event_id: String,
    pub event_timestamp: String,
    pub event_details: EventDetails,
}
