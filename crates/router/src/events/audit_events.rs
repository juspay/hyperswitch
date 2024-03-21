use error_stack::{IntoReport, ResultExt};
use events::{Event, EventInfo};
use serde::Serialize;
use time::PrimitiveDateTime;

#[derive(Debug, Clone, Serialize)]
pub enum AuditEventType {
    Error { error_message: String },
    PaymentCreated,
    ConnectorDecided,
    ConnectorCalled,
    RefundCreated,
    RefundSuccess,
    RefundFail,
    PaymentCancelled { cancellation_reason: Option<String> },
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent {
    event_type: AuditEventType,
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
            AuditEventType::ConnectorDecided => "connector_decided",
            AuditEventType::ConnectorCalled => "connector_called",
            AuditEventType::RefundCreated => "refund_created",
            AuditEventType::RefundSuccess => "refund_success",
            AuditEventType::RefundFail => "refund_fail",
            AuditEventType::PaymentCancelled { .. } => "payment_cancelled",
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
    fn data(&self) -> error_stack::Result<serde_json::Value, events::EventsError> {
        serde_json::to_value(self)
            .into_report()
            .change_context(events::EventsError::SerializationError)
    }

    fn key(&self) -> String {
        "event".to_string()
    }
}
