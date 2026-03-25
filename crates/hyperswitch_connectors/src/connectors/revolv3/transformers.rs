use hyperswitch_domain_models::router_data::ConnectorAuthType;
use hyperswitch_interfaces::errors;
use serde::{Deserialize, Serialize};

pub struct Revolv3AuthType {
    pub api_key: hyperswitch_masking::Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for Revolv3AuthType {
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Revolv3WebhookBody {
    pub body: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Revolv3InvoiceWebhookBody {
    pub invoice: Revolv3WebhookInvoiceData,
    pub event_date_time: Option<String>,
    pub event_type: Option<String>,
    pub revolv_merchant_id: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Revolv3WebhookInvoiceData {
    pub invoice_id: i64,
    pub invoice_status: WebhookInvoiceStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum WebhookInvoiceStatus {
    Paid,
    MerchantPaid,
    Void,
    Pending,
    Recycle,
    Noncollectable,
    Refund,
    MerchantCancelled,
    OneTimePaymentPending,
    PartialRefund,
    BatchPending,
    CapturePending,
    RefundPending,
    RefundDeclined,
    RefundFailed,
    Failed,
}

impl WebhookInvoiceStatus {
    pub fn is_refund_event(&self) -> bool {
        matches!(
            self,
            Self::Refund | Self::PartialRefund | Self::RefundDeclined | Self::RefundFailed
        )
    }
}
