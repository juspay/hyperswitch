use api_models::payments::Amount;
use common_utils::types::MinorUnit;
use diesel_models::fraud_check::FraudCheck;
use events::{Event, EventInfo};
use serde::Serialize;
use time::PrimitiveDateTime;
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event_type")]
pub enum AuditEventType {
    Error {
        error_message: String,
    },
    PaymentCreated,
    ConnectorDecided,
    ConnectorCalled,
    RefundCreated,
    RefundSuccess,
    RefundFail,
    PaymentConfirm {
        client_src: Option<String>,
        client_ver: Option<String>,
        frm_message: Box<Option<FraudCheck>>,
    },
    PaymentCancelled {
        cancellation_reason: Option<String>,
    },
    PaymentCapture {
        capture_amount: Option<MinorUnit>,
        multiple_capture_count: Option<i16>,
    },
    PaymentUpdate {
        amount: Amount,
    },
    PaymentApprove,
    PaymentCreate,
    PaymentStatus,
    PaymentCompleteAuthorize,
    PaymentReject {
        error_code: Option<String>,
        error_message: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent {
    #[serde(flatten)]
    event_type: AuditEventType,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    created_at: PrimitiveDateTime,
}

impl AuditEvent {
    pub fn new(event_type: AuditEventType) -> Self {
        Self {
            event_type,
            created_at: common_utils::date_time::now(),
        }
    }
}

impl Event for AuditEvent {
    type EventType = super::EventType;

    fn timestamp(&self) -> PrimitiveDateTime {
        self.created_at
    }

    fn identifier(&self) -> String {
        let event_type = match &self.event_type {
            AuditEventType::Error { .. } => "error",
            AuditEventType::PaymentCreated => "payment_created",
            AuditEventType::PaymentConfirm { .. } => "payment_confirm",
            AuditEventType::ConnectorDecided => "connector_decided",
            AuditEventType::ConnectorCalled => "connector_called",
            AuditEventType::PaymentCapture { .. } => "payment_capture",
            AuditEventType::RefundCreated => "refund_created",
            AuditEventType::RefundSuccess => "refund_success",
            AuditEventType::RefundFail => "refund_fail",
            AuditEventType::PaymentCancelled { .. } => "payment_cancelled",
            AuditEventType::PaymentUpdate { .. } => "payment_update",
            AuditEventType::PaymentApprove { .. } => "payment_approve",
            AuditEventType::PaymentCreate { .. } => "payment_create",
            AuditEventType::PaymentStatus { .. } => "payment_status",
            AuditEventType::PaymentCompleteAuthorize => "payment_complete_authorize",
            AuditEventType::PaymentReject { .. } => "payment_rejected",
        };
        format!(
            "{event_type}-{}",
            self.timestamp().assume_utc().unix_timestamp_nanos()
        )
    }

    fn class(&self) -> Self::EventType {
        super::EventType::AuditEvent
    }
}

impl EventInfo for AuditEvent {
    type Data = Self;

    fn data(&self) -> error_stack::Result<Self::Data, events::EventsError> {
        Ok(self.clone())
    }

    fn key(&self) -> String {
        "event".to_string()
    }
}
